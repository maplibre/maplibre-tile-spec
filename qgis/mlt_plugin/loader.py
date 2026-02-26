"""Loads MLT files into QGIS memory layers using mlt."""

from collections import defaultdict
from pathlib import Path
from typing import Dict, List, Optional, Tuple

from qgis.core import (
    Qgis,
    QgsFeature,
    QgsField,
    QgsFields,
    QgsGeometry,
    QgsMessageLog,
    QgsProject,
    QgsVectorLayer,
    QgsWkbTypes,
)
from qgis.PyQt.QtCore import QVariant

try:
    import mlt
except ImportError:
    mlt = None

PLUGIN_NAME = "MLT Provider"

GEOM_TYPE_MAP = {
    "Point": QgsWkbTypes.MultiPoint,
    "LineString": QgsWkbTypes.MultiLineString,
    "Polygon": QgsWkbTypes.MultiPolygon,
    "MultiPoint": QgsWkbTypes.MultiPoint,
    "MultiLineString": QgsWkbTypes.MultiLineString,
    "MultiPolygon": QgsWkbTypes.MultiPolygon,
}

_PROMOTE_GEOM = {
    "Point": "MultiPoint",
    "LineString": "MultiLineString",
    "Polygon": "MultiPolygon",
}


def _canonical_geom_type(geom_type_str: str) -> str:
    """Map single-part types to their multi- equivalent for consistent grouping."""
    return _PROMOTE_GEOM.get(geom_type_str, geom_type_str)

QVARIANT_MAP = {
    bool: QVariant.Bool,
    int: QVariant.LongLong,
    float: QVariant.Double,
    str: QVariant.String,
}


def _ensure_mlt():
    if mlt is None:
        raise ImportError(
            "mlt module not found. "
            "Install it with: pip install mlt  "
            "(or build from rust/mlt-pyo3 with maturin)"
        )


def _infer_field_type(value) -> int:
    return QVARIANT_MAP.get(type(value), QVariant.String)


def _qgs_geom_type_string(wkb_type) -> str:
    mapping = {
        QgsWkbTypes.Point: "Point",
        QgsWkbTypes.LineString: "LineString",
        QgsWkbTypes.Polygon: "Polygon",
        QgsWkbTypes.MultiPoint: "MultiPoint",
        QgsWkbTypes.MultiLineString: "MultiLineString",
        QgsWkbTypes.MultiPolygon: "MultiPolygon",
    }
    return mapping.get(wkb_type, "Point")


def _group_features_by_geom_type(mlt_layer):
    groups = {}
    for feat in mlt_layer.features:
        gt = _canonical_geom_type(feat.geometry_type)
        groups.setdefault(gt, []).append(feat)
    return groups


def _discover_fields(features) -> QgsFields:
    field_types = {}
    for feat in features:
        props = feat.properties
        for key, val in props.items():
            if val is not None and key not in field_types:
                field_types[key] = _infer_field_type(val)

    fields = QgsFields()
    for name, vtype in sorted(field_types.items()):
        fields.append(QgsField(name, vtype))
    return fields


def _make_qgs_features(features, vl, field_names) -> List[QgsFeature]:
    qgs_features = []
    for feat in features:
        qf = QgsFeature(vl.fields())
        geom = QgsGeometry()
        geom.fromWkb(bytes(feat.wkb))
        geom.convertToMultiType()
        qf.setGeometry(geom)
        for name in field_names:
            val = feat.properties.get(name)
            if val is not None:
                qf.setAttribute(name, val)
        qgs_features.append(qf)
    return qgs_features


def _decode_file(file_path, zxy=None, tms=True):
    data = Path(file_path).read_bytes()
    if zxy is not None:
        return mlt.decode_mlt(data, z=zxy[0], x=zxy[1], y=zxy[2], tms=tms)
    return mlt.decode_mlt(data)


def _create_and_populate_layer(
    layer_name: str,
    geom_type_str: str,
    features: list,
    crs: str = "EPSG:3857",
) -> Optional[QgsVectorLayer]:
    """Create a memory layer, populate it with features, and add to the project."""
    wkb_type = GEOM_TYPE_MAP.get(geom_type_str)
    if wkb_type is None:
        QgsMessageLog.logMessage(
            f"Skipping unknown geometry type: {geom_type_str}",
            PLUGIN_NAME,
            Qgis.Warning,
        )
        return None

    fields = _discover_fields(features)
    geom_str = _qgs_geom_type_string(wkb_type)

    uri = f"{geom_str}?crs={crs}&index=yes" if crs else f"{geom_str}?index=yes"
    vl = QgsVectorLayer(uri, layer_name, "memory")
    pr = vl.dataProvider()

    pr.addAttributes(fields.toList())
    vl.updateFields()

    field_names = [fields.at(i).name() for i in range(fields.count())]
    pr.addFeatures(_make_qgs_features(features, vl, field_names))
    vl.updateExtents()

    QgsProject.instance().addMapLayer(vl)
    QgsMessageLog.logMessage(
        f"Loaded layer '{layer_name}' with {len(features)} features",
        PLUGIN_NAME,
        Qgis.Info,
    )
    return vl


# ── Public API ────────────────────────────────────────────────────────


def load_mlt_file(
    file_path: str,
    zxy: Optional[Tuple[int, int, int]] = None,
    tms: bool = True,
) -> List[QgsVectorLayer]:
    """Read a single MLT file and add its layers to the QGIS project."""
    _ensure_mlt()

    mlt_layers = _decode_file(file_path, zxy=zxy, tms=tms)
    stem = Path(file_path).stem
    result = []

    for mlt_layer in mlt_layers:
        groups = _group_features_by_geom_type(mlt_layer)
        for geom_type_str, features in groups.items():
            name = f"{stem} \u2014 {mlt_layer.name}"
            if len(groups) > 1:
                name += f" ({geom_type_str})"
            crs = "EPSG:3857" if zxy is not None else ""
            vl = _create_and_populate_layer(name, geom_type_str, features, crs=crs)
            if vl:
                result.append(vl)

    return result


def load_mlt_files_merged(
    file_paths: List[str],
    file_coords: Optional[Dict[str, Tuple[int, int, int]]] = None,
    tms: bool = True,
) -> List[QgsVectorLayer]:
    """Read multiple MLT files and merge same-named layers into single QGIS layers.

    Features from every tile that share the same MLT layer name and geometry
    type are combined into one QGIS memory layer, giving a seamless multi-tile
    view.

    Args:
        file_paths: List of .mlt file paths.
        file_coords: Optional dict {path: (z, x, y)} for geo-referencing.
                     Files not in the dict (or if None) use raw tile coords.
        tms: TMS y-axis convention (default True).
    """
    _ensure_mlt()

    # Key: (layer_name, geom_type_str) -> list of mlt features
    buckets: Dict[Tuple[str, str], list] = defaultdict(list)

    for path in file_paths:
        zxy = file_coords.get(path) if file_coords else None
        mlt_layers = _decode_file(path, zxy=zxy, tms=tms)

        for mlt_layer in mlt_layers:
            for feat in mlt_layer.features:
                bucket_key = (mlt_layer.name, _canonical_geom_type(feat.geometry_type))
                buckets[bucket_key].append(feat)

    result = []
    n_files = len(file_paths)
    label = f"{n_files} tiles"

    for (layer_name, geom_type_str), features in sorted(buckets.items()):
        display_name = f"{label} \u2014 {layer_name}"
        # Only disambiguate by geom type if the same layer name has multiple types
        sibling_types = [
            gt for (ln, gt) in buckets if ln == layer_name
        ]
        if len(sibling_types) > 1:
            display_name += f" ({geom_type_str})"

        crs = "EPSG:3857" if file_coords else ""
        vl = _create_and_populate_layer(display_name, geom_type_str, features, crs=crs)
        if vl:
            result.append(vl)

    return result
