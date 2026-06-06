"""Behavior tests for maplibre_tiles.encode().

Each test round-trips through the public interface: encode() -> decode_mlt()
and/or decode_mlt_to_geojson(), so the suite is a specification of observable
behavior, not of the encoder's internals.
"""

import json

import pytest

import maplibre_tiles as mlt


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
