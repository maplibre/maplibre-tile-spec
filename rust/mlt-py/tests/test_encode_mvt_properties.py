import json

import mapbox_vector_tile
from hypothesis import given, settings
from hypothesis import strategies as st

import maplibre_tiles as mlt

EXTENT = 4096

# y_coord_down keeps coords in MVT tile space; otherwise the input is read as y-up and flipped.
_OPTS = {"extents": EXTENT, "y_coord_down": True}

_coord = st.integers(min_value=0, max_value=EXTENT - 1)
_u64 = st.integers(min_value=0, max_value=2**64 - 1)


def _encode(features):
    """Build an MVT tile of POINT features, then encode it to MLT."""
    return mlt.encode_mvt(
        mapbox_vector_tile.encode({"name": "l", "features": features}, default_options=_OPTS)
    )


@given(points=st.lists(st.tuples(_coord, _coord), min_size=1, max_size=10))
@settings(max_examples=300)
def test_point_geometry_roundtrips(points):
    # MLT reorders features, so key on id, not position.
    blob = _encode(
        [{"geometry": f"POINT({x} {y})", "id": i} for i, (x, y) in enumerate(points)]
    )

    decoded = mlt.decode_mlt(blob)[0].features
    assert len(decoded) == len(points)
    geom_by_id = {
        f["id"]: f["geometry"] for f in json.loads(mlt.decode_mlt_to_geojson(blob))["features"]
    }
    for i, (x, y) in enumerate(points):
        assert geom_by_id[i] == {"type": "Point", "coordinates": [x, y]}


# One type per tile: mapbox_vector_tile dedups the value pool by Python equality,
# and 0 == 0.0 == False, so mixing those types in one tile collapses the columns.
_scalar_columns = {
    "bool": st.booleans(),
    "int": st.integers(min_value=-(2**63), max_value=2**63 - 1),
    "float": st.floats(allow_nan=False, allow_infinity=False),
    "str": st.text(max_size=12),
}


def _check_column(values):
    blob = _encode(
        [{"geometry": "POINT(0 0)", "properties": {"v": v}, "id": i} for i, v in enumerate(values)]
    )
    by_id = {feat.id: feat for feat in mlt.decode_mlt(blob)[0].features}
    for i, value in enumerate(values):
        got = by_id[i].properties["v"]
        assert got == value
        # bool must not be silently widened to int (True == 1 in Python).
        assert isinstance(got, type(value))


@given(values=st.lists(_scalar_columns["bool"], min_size=1, max_size=10))
@settings(max_examples=200)
def test_bool_properties_roundtrip(values):
    _check_column(values)


@given(values=st.lists(_scalar_columns["int"], min_size=1, max_size=10))
@settings(max_examples=300)
def test_int_properties_roundtrip(values):
    _check_column(values)


@given(values=st.lists(_scalar_columns["float"], min_size=1, max_size=10))
@settings(max_examples=300)
def test_float_properties_roundtrip(values):
    _check_column(values)


@given(values=st.lists(_scalar_columns["str"], min_size=1, max_size=10))
@settings(max_examples=300)
def test_str_properties_roundtrip(values):
    _check_column(values)


@given(feature_id=_u64)
@settings(max_examples=200)
def test_id_roundtrips_for_any_u64(feature_id):
    blob = _encode([{"geometry": "POINT(1 2)", "id": feature_id}])
    assert mlt.decode_mlt(blob)[0].features[0].id == feature_id
