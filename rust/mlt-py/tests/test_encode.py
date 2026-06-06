"""Behavior tests for maplibre_tiles.encode().

Each test round-trips through the public interface: encode() -> decode_mlt()
and/or decode_mlt_to_geojson(), so the suite is a specification of observable
behavior, not of the encoder's internals.
"""

import json
import struct

import pytest

import maplibre_tiles as mlt


def _wkb_point(x, y):
    return struct.pack("<BIdd", 1, 1, float(x), float(y))


def _wkb_linestring(coords):
    out = struct.pack("<BII", 1, 2, len(coords))
    for x, y in coords:
        out += struct.pack("<dd", float(x), float(y))
    return out


def _wkb_point_z(x, y, z):
    # ISO WKB PointZ: geometry type 1001.
    return struct.pack("<BIddd", 1, 1001, float(x), float(y), float(z))


def test_point_layer_dict_roundtrips():
    blob = mlt.encode(
        {
            "name": "roads",
            "extent": 4096,
            "features": [
                {"geometry": {"type": "Point", "coordinates": [2048, 1024]}},
            ],
        }
    )
    assert isinstance(blob, bytes)

    layers = mlt.decode_mlt(blob)
    assert len(layers) == 1
    layer = layers[0]
    assert layer.name == "roads"
    assert layer.extent == 4096
    assert len(layer.features) == 1
    assert layer.features[0].geometry_type == "Point"

    fc = json.loads(mlt.decode_mlt_to_geojson(blob))
    assert fc["features"][0]["geometry"] == {
        "type": "Point",
        "coordinates": [2048, 1024],
    }


def test_scalar_properties_roundtrip():
    blob = mlt.encode(
        {
            "name": "roads",
            "extent": 4096,
            "features": [
                {
                    "geometry": {"type": "Point", "coordinates": [1, 2]},
                    "properties": {
                        "name": "main",
                        "lanes": 3,
                        "oneway": True,
                        "width": 3.5,
                    },
                }
            ],
        }
    )
    props = mlt.decode_mlt(blob)[0].features[0].properties
    assert props["name"] == "main"
    assert props["lanes"] == 3
    assert props["oneway"] is True
    assert props["width"] == 3.5


def test_feature_id_roundtrips():
    blob = mlt.encode(
        {
            "name": "roads",
            "extent": 4096,
            "features": [
                {"id": 42, "geometry": {"type": "Point", "coordinates": [1, 2]}},
                {"geometry": {"type": "Point", "coordinates": [3, 4]}},
            ],
        }
    )
    feats = mlt.decode_mlt(blob)[0].features
    assert feats[0].id == 42
    assert feats[1].id is None


@pytest.mark.parametrize(
    "geometry",
    [
        {"type": "Point", "coordinates": [2048, 1024]},
        {"type": "MultiPoint", "coordinates": [[1, 1], [2, 2]]},
        {"type": "LineString", "coordinates": [[0, 0], [10, 0], [10, 10]]},
        {
            "type": "MultiLineString",
            "coordinates": [[[0, 0], [1, 1]], [[2, 2], [3, 3]]],
        },
        {
            "type": "Polygon",
            "coordinates": [
                [[0, 0], [10, 0], [10, 10], [0, 10], [0, 0]],
                [[2, 2], [4, 2], [4, 4], [2, 4], [2, 2]],
            ],
        },
        {
            "type": "MultiPolygon",
            "coordinates": [
                [[[0, 0], [5, 0], [5, 5], [0, 5], [0, 0]]],
                [[[6, 6], [8, 6], [8, 8], [6, 8], [6, 6]]],
            ],
        },
    ],
    ids=lambda g: g["type"],
)
def test_geometry_kinds_roundtrip(geometry):
    blob = mlt.encode(
        {"name": "l", "extent": 4096, "features": [{"geometry": geometry}]}
    )
    fc = json.loads(mlt.decode_mlt_to_geojson(blob))
    assert fc["features"][0]["geometry"] == geometry


@pytest.mark.parametrize(
    "wkt,expected",
    [
        ("POINT (2048 1024)", {"type": "Point", "coordinates": [2048, 1024]}),
        (
            "LINESTRING (0 0, 10 0, 10 10)",
            {"type": "LineString", "coordinates": [[0, 0], [10, 0], [10, 10]]},
        ),
        (
            "POLYGON ((0 0, 10 0, 10 10, 0 10, 0 0))",
            {
                "type": "Polygon",
                "coordinates": [[[0, 0], [10, 0], [10, 10], [0, 10], [0, 0]]],
            },
        ),
    ],
    ids=["point", "linestring", "polygon"],
)
def test_wkt_geometry_roundtrips(wkt, expected):
    blob = mlt.encode(
        {"name": "l", "extent": 4096, "features": [{"geometry": wkt}]}
    )
    fc = json.loads(mlt.decode_mlt_to_geojson(blob))
    assert fc["features"][0]["geometry"] == expected


@pytest.mark.parametrize(
    "wkb,expected",
    [
        (_wkb_point(2048, 1024), {"type": "Point", "coordinates": [2048, 1024]}),
        (
            _wkb_linestring([(0, 0), (10, 0), (10, 10)]),
            {"type": "LineString", "coordinates": [[0, 0], [10, 0], [10, 10]]},
        ),
    ],
    ids=["point", "linestring"],
)
def test_wkb_geometry_roundtrips(wkb, expected):
    blob = mlt.encode(
        {"name": "l", "extent": 4096, "features": [{"geometry": wkb}]}
    )
    fc = json.loads(mlt.decode_mlt_to_geojson(blob))
    assert fc["features"][0]["geometry"] == expected


def test_feature_collection_input_takes_name_extent_from_options():
    fc_in = {
        "type": "FeatureCollection",
        "features": [
            {
                "type": "Feature",
                "geometry": {"type": "Point", "coordinates": [1, 2]},
                "properties": {"k": "v"},
            }
        ],
    }
    blob = mlt.encode(fc_in, {"name": "places", "extent": 8192})
    layer = mlt.decode_mlt(blob)[0]
    assert layer.name == "places"
    assert layer.extent == 8192
    assert layer.features[0].properties["k"] == "v"


@pytest.mark.parametrize(
    "options",
    [
        {"sort": "none"},
        {"sort": "auto"},
        {"sort": "morton"},
        {"sort": "hilbert"},
        {"sort": "id"},
        {"tessellate": True},
        {"allow_fsst": False, "allow_fpf": False, "allow_shared_dict": False},
    ],
)
def test_valid_options_preserve_features(options):
    base = {
        "name": "roads",
        "extent": 4096,
        "features": [
            {"id": 1, "geometry": {"type": "Point", "coordinates": [1, 2]}, "properties": {"k": "a"}},
            {"id": 2, "geometry": {"type": "Point", "coordinates": [3, 4]}, "properties": {"k": "b"}},
        ],
    }
    layer = mlt.decode_mlt(mlt.encode(base, options))[0]
    # sorting may reorder features, so compare as an unordered set.
    got = {(f.id, f.properties["k"]) for f in layer.features}
    assert got == {(1, "a"), (2, "b")}


def test_invalid_sort_option_errors():
    base = {"name": "r", "features": [{"geometry": {"type": "Point", "coordinates": [1, 2]}}]}
    with pytest.raises(ValueError):
        mlt.encode(base, {"sort": "bogus"})


def test_multi_layer_tile_via_concatenation():
    roads = mlt.encode(
        {"name": "roads", "extent": 4096, "features": [{"id": 1, "geometry": {"type": "Point", "coordinates": [1, 2]}}]}
    )
    water = mlt.encode(
        {"name": "water", "extent": 4096, "features": [{"id": 2, "geometry": {"type": "LineString", "coordinates": [[0, 0], [5, 5]]}}]}
    )
    tile = b"".join([roads, water])
    layers = mlt.decode_mlt(tile)
    assert [layer.name for layer in layers] == ["roads", "water"]
    assert mlt.list_layers(tile) == ["roads", "water"]


@pytest.mark.parametrize(
    "feature",
    [
        {"geometry": {"type": "Point", "coordinates": [1, 2]}, "properties": {"tags": {"a": 1}}},
        {"geometry": {"type": "Point", "coordinates": [1, 2]}, "properties": {"tags": [1, 2]}},
        {"geometry": {"type": "Point", "coordinates": [1, 2]}, "id": "way/1"},
        {"geometry": {"type": "Point", "coordinates": [1, 2]}, "id": -1},
        {"geometry": {"type": "Point", "coordinates": [1, 2]}, "id": 3.5},
        {"geometry": None},
        {"geometry": {"type": "LineString", "coordinates": []}},
        {"geometry": {"type": "Point", "coordinates": [2048.5, 1024]}},
        {"geometry": {"type": "Point", "coordinates": [1, 2, 3]}},
    ],
    ids=[
        "nested_dict_prop",
        "nested_list_prop",
        "string_id",
        "negative_id",
        "float_id",
        "null_geometry",
        "empty_geometry",
        "fractional_coord",
        "three_d_coord",
    ],
)
def test_strict_feature_errors(feature):
    with pytest.raises(ValueError):
        mlt.encode({"name": "r", "extent": 4096, "features": [feature]})


def test_empty_layer_errors():
    with pytest.raises(ValueError):
        mlt.encode({"name": "r", "extent": 4096, "features": []})


def test_feature_collection_without_name_errors():
    fc = {
        "type": "FeatureCollection",
        "features": [{"geometry": {"type": "Point", "coordinates": [1, 2]}}],
    }
    with pytest.raises(ValueError):
        mlt.encode(fc)


def test_wkt_3d_geometry_errors():
    with pytest.raises(ValueError):
        mlt.encode(
            {"name": "l", "extent": 4096, "features": [{"geometry": "POINT Z (1 2 3)"}]}
        )


def test_wkb_3d_geometry_errors():
    with pytest.raises(ValueError):
        mlt.encode(
            {"name": "l", "extent": 4096, "features": [{"geometry": _wkb_point_z(1, 2, 3)}]}
        )


def test_wkt_empty_point_errors():
    # geo_traits' to_geometry() panics on an empty point; encode must turn that
    # into a clean ValueError, not a PanicException.
    with pytest.raises(ValueError):
        mlt.encode(
            {"name": "l", "extent": 4096, "features": [{"geometry": "POINT EMPTY"}]}
        )
