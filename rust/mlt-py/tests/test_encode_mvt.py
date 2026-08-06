from pathlib import Path

import pytest

import maplibre_tiles as mlt

FIXTURES = Path(__file__).resolve().parents[3] / "test" / "fixtures"

# Tile{ layer{ name:"L", version:2, extent:4096, features:[] } }
_EMPTY_LAYER_MVT = bytes([0x1A, 0x08, 0x0A, 0x01, 0x4C, 0x78, 0x02, 0x28, 0x80, 0x20])


def test_round_trip_single_point_layer():
    mvt = (FIXTURES / "simple" / "point-boolean.mvt").read_bytes()

    out = mlt.encode_mvt(mvt)

    assert isinstance(out, bytes)
    assert len(out) > 0

    layers = mlt.decode_mlt(out)
    assert len(layers) == 1
    assert len(layers[0].features) >= 1


def test_multi_layer_tile_preserves_every_layer():
    mvt = (FIXTURES / "omt" / "10_530_682.mvt").read_bytes()

    names = mlt.list_layers(mlt.encode_mvt(mvt))

    assert names == [
        "water",
        "waterway",
        "landcover",
        "landuse",
        "mountain_peak",
        "park",
        "aeroway",
        "transportation",
        "transportation_name",
        "place",
        "aerodrome_label",
    ]


def test_invalid_bytes_raise_value_error():
    with pytest.raises(ValueError):
        mlt.encode_mvt(b"this is not a protobuf tile")


def test_zero_feature_layer_is_dropped_not_crashed():
    out = mlt.encode_mvt(_EMPTY_LAYER_MVT)

    assert out == b""
    assert mlt.list_layers(out) == []


def test_empty_layer_alongside_real_layer_is_dropped():
    real = (FIXTURES / "simple" / "point-boolean.mvt").read_bytes()

    out = mlt.encode_mvt(real + _EMPTY_LAYER_MVT)

    assert mlt.list_layers(out) == ["layer"]
