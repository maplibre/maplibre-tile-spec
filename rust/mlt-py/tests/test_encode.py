"""Behavior tests for maplibre_tiles.encode().

encode() takes a GeoJSON FeatureCollection plus the MLT layer name/extent.
Each test round-trips through the public interface: encode() -> decode_mlt() and/or decode_mlt_to_geojson().
The suite is a specification of observable behavior, not the encoder's internals.
"""

import json

import pytest

import maplibre_tiles as mlt

POINT = {"type": "Point", "coordinates": [2048, 1024]}


def _fc(features):
    return {"type": "FeatureCollection", "features": features}


def _feature(geometry, **members):
    return {"type": "Feature", "geometry": geometry, **members}


def test_point_feature_collection_roundtrips():
    blob = mlt.encode(_fc([_feature(POINT)]), name="roads", extent=4096)
    assert isinstance(blob, bytes)

    layers = mlt.decode_mlt(blob)
    assert len(layers) == 1
    layer = layers[0]
    assert layer.name == "roads"
    assert layer.extent == 4096
    assert len(layer.features) == 1
    assert layer.features[0].geometry_type == "Point"

    fc = json.loads(mlt.decode_mlt_to_geojson(blob))
    assert fc["features"][0]["geometry"] == POINT


def test_extent_defaults_to_4096():
    blob = mlt.encode(_fc([_feature(POINT)]), name="roads")
    assert mlt.decode_mlt(blob)[0].extent == 4096


def test_scalar_properties_roundtrip():
    blob = mlt.encode(
        _fc(
            [
                _feature(
                    {"type": "Point", "coordinates": [1, 2]},
                    properties={
                        "name": "main",
                        "lanes": 3,
                        "oneway": True,
                        "width": 3.5,
                    },
                )
            ]
        ),
        name="roads",
    )
    props = mlt.decode_mlt(blob)[0].features[0].properties
    assert props["name"] == "main"
    assert props["lanes"] == 3
    assert props["oneway"] is True
    assert props["width"] == 3.5


def test_feature_id_roundtrips():
    blob = mlt.encode(
        _fc(
            [
                _feature({"type": "Point", "coordinates": [1, 2]}, id=42),
                _feature({"type": "Point", "coordinates": [3, 4]}),
            ]
        ),
        name="roads",
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
    blob = mlt.encode(_fc([_feature(geometry)]), name="l")
    fc = json.loads(mlt.decode_mlt_to_geojson(blob))
    assert fc["features"][0]["geometry"] == geometry


def test_multi_layer_tile_via_concatenation():
    roads = mlt.encode(
        _fc([_feature({"type": "Point", "coordinates": [1, 2]}, id=1)]),
        name="roads",
    )
    water = mlt.encode(
        _fc([_feature({"type": "LineString", "coordinates": [[0, 0], [5, 5]]}, id=2)]),
        name="water",
    )
    tile = b"".join([roads, water])
    layers = mlt.decode_mlt(tile)
    assert [layer.name for layer in layers] == ["roads", "water"]
    assert mlt.list_layers(tile) == ["roads", "water"]


@pytest.mark.parametrize(
    "feature",
    [
        _feature({"type": "Point", "coordinates": [1, 2]}, properties={"tags": {"a": 1}}),
        _feature({"type": "Point", "coordinates": [1, 2]}, properties={"tags": [1, 2]}),
        _feature({"type": "Point", "coordinates": [1, 2]}, id="way/1"),
        _feature({"type": "Point", "coordinates": [1, 2]}, id=-1),
        _feature({"type": "Point", "coordinates": [1, 2]}, id=3.5),
        _feature(None),
        _feature({"type": "LineString", "coordinates": []}),
        _feature({"type": "Point", "coordinates": [2048.5, 1024]}),
        _feature({"type": "Point", "coordinates": [1, 2, 3]}),
        {"geometry": {"type": "Point", "coordinates": [1, 2]}},  # missing "type": "Feature"
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
        "not_a_feature",
    ],
)
def test_strict_feature_errors(feature):
    with pytest.raises(ValueError):
        mlt.encode(_fc([feature]), name="r")


def test_non_feature_collection_input_errors():
    # A bare Feature (not wrapped in a FeatureCollection) is rejected.
    with pytest.raises(ValueError):
        mlt.encode(_feature({"type": "Point", "coordinates": [1, 2]}), name="r")


def test_empty_layer_errors():
    with pytest.raises(ValueError):
        mlt.encode(_fc([]), name="r")
