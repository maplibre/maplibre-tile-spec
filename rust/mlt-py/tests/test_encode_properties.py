"""Property-based round-trip tests for maplibre_tiles.encode().

These exercise the universally-quantified invariants that the example tests in test_encode.py only sample.
For any valid input, encode() followed by a decode preserves the geometry, id, and property values.

Generator constraints (each reflects a real precondition of the encoder):
  * Coordinates are confined to [0, EXTENT) — tile-local space.
    Arbitrary i32 values are not in scope, since spatial sorts assume in-tile coordinates.
  * Features get a unique id (their index) so they can be matched back after the encoder reorders them under spatial/id sorting.
  * Within one feature each property key is distinct, so no cross-feature type widening is involved.
"""

import json

from hypothesis import given, settings
from hypothesis import strategies as st

import maplibre_tiles as mlt

EXTENT = 4096

_coord = st.integers(min_value=0, max_value=EXTENT - 1)
_position = st.tuples(_coord, _coord).map(list)

_point = _position.map(lambda c: {"type": "Point", "coordinates": c})
_linestring = st.lists(_position, min_size=2, max_size=8).map(
    lambda cs: {"type": "LineString", "coordinates": cs}
)
_multipoint = st.lists(_position, min_size=1, max_size=8).map(
    lambda cs: {"type": "MultiPoint", "coordinates": cs}
)
# A closed ring: >=3 distinct positions with the first repeated at the end.
_ring = st.lists(_position, min_size=3, max_size=6).map(lambda pts: pts + [pts[0]])
_polygon = st.lists(_ring, min_size=1, max_size=3).map(
    lambda rings: {"type": "Polygon", "coordinates": rings}
)
_multipolygon = st.lists(st.lists(_ring, min_size=1, max_size=2), min_size=1, max_size=3).map(
    lambda polys: {"type": "MultiPolygon", "coordinates": polys}
)
_geometry = st.one_of(_point, _linestring, _multipoint, _polygon, _multipolygon)

# Scalar property values spanning every supported column type and the i64/u64 width boundary.
_scalar = st.one_of(
    st.booleans(),
    st.integers(min_value=-(2**63), max_value=2**64 - 1),
    st.floats(allow_nan=False, allow_infinity=False),
    st.text(max_size=12),
)
_key = st.text(
    alphabet=st.characters(min_codepoint=ord("a"), max_codepoint=ord("z")),
    min_size=1,
    max_size=8,
)


def _fc(features):
    return {"type": "FeatureCollection", "features": features}


@given(geometries=st.lists(_geometry, min_size=1, max_size=10))
@settings(max_examples=300)
def test_geometry_roundtrips_for_any_input(geometries):
    features = [
        {"type": "Feature", "id": i, "geometry": g} for i, g in enumerate(geometries)
    ]
    blob = mlt.encode(_fc(features), name="l", extent=EXTENT)

    fc = json.loads(mlt.decode_mlt_to_geojson(blob))
    by_id = {f["id"]: f["geometry"] for f in fc["features"]}

    assert len(by_id) == len(geometries)
    for i, geometry in enumerate(geometries):
        assert by_id[i] == geometry


@given(feature_id=st.integers(min_value=0, max_value=2**64 - 1))
@settings(max_examples=200)
def test_id_roundtrips_for_any_u64(feature_id):
    blob = mlt.encode(
        _fc(
            [
                {
                    "type": "Feature",
                    "id": feature_id,
                    "geometry": {"type": "Point", "coordinates": [1, 2]},
                }
            ]
        ),
        name="l",
        extent=EXTENT,
    )
    assert mlt.decode_mlt(blob)[0].features[0].id == feature_id


@given(props=st.dictionaries(_key, _scalar, max_size=8))
@settings(max_examples=300)
def test_scalar_properties_roundtrip_for_any_dict(props):
    blob = mlt.encode(
        _fc(
            [
                {
                    "type": "Feature",
                    "geometry": {"type": "Point", "coordinates": [1, 2]},
                    "properties": props,
                }
            ]
        ),
        name="l",
        extent=EXTENT,
    )
    got = mlt.decode_mlt(blob)[0].features[0].properties
    for key, value in props.items():
        assert got[key] == value
        # bool must not be silently widened to int (True == 1 in Python).
        assert isinstance(got[key], type(value))
