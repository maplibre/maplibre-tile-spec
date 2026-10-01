import json
import struct
from pathlib import Path

import pytest

import maplibre_tiles as mlt

SYNTHETIC = Path(__file__).parents[3] / "test" / "synthetic" / "0x02"
Z_FIXTURES = sorted(SYNTHETIC.glob("z_*.mlt"))
WKB_TYPES = {
    1: "Point",
    2: "LineString",
    3: "Polygon",
    4: "MultiPoint",
    5: "MultiLineString",
    6: "MultiPolygon",
}


def _read_wkb(data, offset=0):
    assert data[offset] == 1
    (code,) = struct.unpack_from("<I", data, offset + 1)
    dims = 3 if code > 1000 else 2
    offset += 5

    def count():
        nonlocal offset
        (n,) = struct.unpack_from("<I", data, offset)
        offset += 4
        return n

    def position():
        nonlocal offset
        pos = list(struct.unpack_from(f"<{dims}d", data, offset))
        offset += 8 * dims
        return pos

    def positions():
        return [position() for _ in range(count())]

    def parts():
        nonlocal offset
        out = []
        for _ in range(count()):
            part, offset = _read_wkb(data, offset)
            out.append(part["coordinates"])
        return out

    kind = WKB_TYPES[code % 1000]
    if kind == "Point":
        coords = position()
    elif kind == "LineString":
        coords = positions()
    elif kind == "Polygon":
        coords = [positions() for _ in range(count())]
    else:
        coords = parts()
    return {"type": kind, "coordinates": coords}, offset


def _wkb_geometry(wkb):
    geometry, end = _read_wkb(wkb)
    assert end == len(wkb)
    return geometry


def _fc(geometry):
    return {
        "type": "FeatureCollection",
        "features": [{"type": "Feature", "geometry": geometry, "properties": {}}],
    }


@pytest.mark.parametrize("fixture", Z_FIXTURES, ids=[f.stem for f in Z_FIXTURES])
def test_z_fixture_wkb_matches_its_geojson(fixture):
    expected = json.loads(fixture.with_suffix(".json").read_text())["features"]
    layers = mlt.decode_mlt(fixture.read_bytes())
    features = [feat for layer in layers for feat in layer.features]
    assert [_wkb_geometry(feat.wkb) for feat in features] == [
        feat["geometry"] for feat in expected
    ]
    assert [layer.z_step for layer in layers for _ in layer.features] == [
        feat["properties"]["_z_step"] for feat in expected
    ]


def test_a_flat_layer_has_no_z_step_and_2d_wkb():
    blob = mlt.encode_geojson(_fc({"type": "Point", "coordinates": [1, 2]}), "r")
    layer = mlt.decode_mlt(blob)[0]
    assert layer.z_step is None
    assert _wkb_geometry(layer.features[0].wkb)["coordinates"] == [1, 2]


def test_a_transformed_z_reads_as_metres():
    blob = mlt.encode_geojson(
        _fc({"type": "Point", "coordinates": [0, 0, 100_120]}), "r", z_step=-1
    )
    wkb = mlt.decode_mlt(blob, z=0, x=0, y=0, tms=False)[0].features[0].wkb
    assert _wkb_geometry(wkb)["coordinates"][2] == 12.0


@pytest.mark.parametrize(
    "geometry",
    [
        {"type": "Point", "coordinates": [1, 2, 3]},
        {"type": "LineString", "coordinates": [[0, 0, 7], [5, 5, -2]]},
        {
            "type": "Polygon",
            "coordinates": [[[0, 0, 1], [10, 0, 2], [10, 10, 3], [0, 0, 1]]],
        },
        {"type": "MultiPoint", "coordinates": [[1, 2, 3], [4, 5, 6]]},
    ],
    ids=["point", "line", "polygon", "multi_point"],
)
def test_z_roundtrips_through_encode_geojson(geometry):
    blob = mlt.encode_geojson(_fc(geometry), "r", z_step=0)
    layer = mlt.decode_mlt(blob)[0]
    assert layer.z_step == 0
    assert _wkb_geometry(layer.features[0].wkb) == geometry
    assert json.loads(mlt.decode_mlt_to_geojson(blob))["features"][0]["geometry"] == geometry


@pytest.mark.parametrize(
    ("geometry", "z_step", "message"),
    [
        ({"type": "Point", "coordinates": [1, 2]}, 0, r"'z_step' needs \[x, y, z\] positions"),
        ({"type": "Point", "coordinates": [1, 2, 3]}, None, r"\[x, y, z\] positions need a 'z_step'"),
        ({"type": "Point", "coordinates": [1, 2, 3]}, 5, r"a z step of 10\^5 m is outside 10\^-3..=10\^4 m"),
    ],
    ids=["z_step_without_z", "z_without_z_step", "z_step_out_of_range"],
)
def test_z_and_z_step_must_agree(geometry, z_step, message):
    with pytest.raises(ValueError, match=f"^{message}$"):
        mlt.encode_geojson(_fc(geometry), "r", z_step=z_step)
