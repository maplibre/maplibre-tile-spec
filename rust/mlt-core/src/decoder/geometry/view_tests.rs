//! Failure contracts of [`GeometryValues::geometry`]: a corrupted offset table is an
//! error when the view is made, with the same error `to_geojson` reports, never a
//! panic while the view is walked.

use geo_types::{Coord, Geometry, LineString, Point};

use crate::MltError;
use crate::decoder::{GeometryType, GeometryValues};

fn line(from: i32, n: i32) -> Geometry<i32> {
    Geometry::LineString(LineString(
        (from..from + n).map(|i| Coord { x: i, y: -i }).collect(),
    ))
}

/// Two line strings of 3 and 2 vertices: `part_offsets == [0, 3, 5]`.
fn lines() -> GeometryValues {
    GeometryValues::default()
        .with_geom(&line(0, 3))
        .with_geom(&line(10, 2))
}

fn points() -> GeometryValues {
    GeometryValues::default()
        .with_geom(&Geometry::Point(Point::new(1, 2)))
        .with_geom(&Geometry::Point(Point::new(3, 4)))
}

/// Both paths must agree on what is wrong with the feature.
fn assert_same_error(values: &GeometryValues, index: usize) -> MltError {
    let view_err = values.geometry(index).expect_err("the view must fail");
    let oracle_err = values.to_geojson(index).expect_err("to_geojson must fail");
    assert_eq!(view_err.to_string(), oracle_err.to_string());
    view_err
}

#[test]
fn an_index_past_the_last_feature_is_an_error() {
    let err = assert_same_error(&lines(), 2);
    assert!(
        matches!(err, MltError::GeometryIndexOutOfBounds(2)),
        "{err}"
    );
}

#[test]
fn a_non_monotonic_offset_pair_is_an_error_not_an_empty_part() {
    let mut values = lines();
    values.part_offsets = Some(vec![0, 3, 1]);
    assert!(values.geometry(0).is_ok(), "feature 0 is intact");
    let err = values.geometry(1).expect_err("start > end must fail");
    assert!(
        matches!(
            err,
            MltError::GeometryOffsetsNotMonotonic {
                index: 1,
                field: "part_offsets",
                idx: 1
            }
        ),
        "{err}"
    );
}

#[test]
fn a_truncated_offset_level_is_out_of_bounds() {
    let mut values = lines();
    values.part_offsets = Some(vec![0, 3]);
    assert!(values.geometry(0).is_ok());
    let err = assert_same_error(&values, 1);
    assert!(
        matches!(
            err,
            MltError::GeometryOutOfBounds {
                index: 1,
                field: "part_offsets",
                idx: 2,
                len: 2
            }
        ),
        "{err}"
    );
}

#[test]
fn a_vertex_past_the_buffer_is_an_error() {
    let mut values = lines();
    values.vertices.as_mut().unwrap().truncate(8); // 4 vertices, feature 1 needs 3..5
    assert!(values.geometry(0).is_ok());
    let err = assert_same_error(&values, 1);
    assert!(
        matches!(
            err,
            MltError::GeometryVertexOutOfBounds {
                index: 1,
                vertex: 4,
                count: 4
            }
        ),
        "{err}"
    );
}

#[test]
fn a_type_that_needs_a_missing_level_is_an_error() {
    let mut values = points();
    values.vector_types[1] = GeometryType::LineString;
    let err = assert_same_error(&values, 1);
    assert!(
        matches!(err, MltError::NoPartOffsets(1, GeometryType::LineString)),
        "{err}"
    );

    let mut values = points();
    values.vector_types[0] = GeometryType::MultiPoint;
    let err = assert_same_error(&values, 0);
    assert!(
        matches!(
            err,
            MltError::NoGeometryOffsets(0, GeometryType::MultiPoint)
        ),
        "{err}"
    );

    let mut values = lines();
    values.vector_types[0] = GeometryType::Polygon;
    let err = assert_same_error(&values, 0);
    assert!(
        matches!(err, MltError::NoRingOffsets(0, GeometryType::Polygon)),
        "{err}"
    );
}

#[test]
fn a_corrupted_feature_leaves_its_neighbours_viewable() {
    let mut values = lines();
    values.part_offsets = Some(vec![0, 3, 1]);
    let ok = values.geometry(0).unwrap();
    assert_eq!(ok.vertex_range(), Some(0..3));
    assert_eq!(ok.vertices().count(), 3);
}
