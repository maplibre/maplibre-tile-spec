use std::collections::HashMap;

use arbitrary::{Arbitrary, Unstructured};
use mlt_core::encoder::SortStrategy::Unsorted;
use mlt_core::encoder::{EncoderConfig, WireVersion, stage_tile};
use mlt_core::geo_types::{
    Coord, Geometry, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon,
};
use mlt_core::{MltError, TileFeature, TileLayer, ZStep};

use crate::roundtrip::{decode, encode_decode};

/// A layer of every geometry type whose vertices carry z, encoded as v2.
///
/// Stripping z must decode to what the layer decodes to without it, z must survive wherever the geometry does,
/// and re-staging the decoded layer must be a fixpoint.
pub struct ZInput {
    pub geometries: Vec<Geometry<i32>>,
    pub step: ZStep,
    pub z: Vec<Vec<i32>>,
    pub config: EncoderConfig,
}

impl Arbitrary<'_> for ZInput {
    fn arbitrary(u: &mut Unstructured<'_>) -> arbitrary::Result<Self> {
        let count = u.int_in_range(1..=16u8)?;
        let geometries: Vec<Geometry<i32>> = (0..count)
            .map(|_| geometry(u))
            .collect::<arbitrary::Result<_>>()?;
        let step = ZStep::new(u.int_in_range(ZStep::MIN_EXPONENT..=ZStep::MAX_EXPONENT)?)
            .map_err(|_| arbitrary::Error::IncorrectFormat)?;
        let z = geometries
            .iter()
            .map(|g| {
                let n = TileFeature::new(g.clone()).vertex_count();
                (0..n).map(|_| u.arbitrary()).collect()
            })
            .collect::<arbitrary::Result<_>>()?;
        let config = EncoderConfig::arbitrary(u)?
            .with_wire_version(WireVersion::V02)
            .with_spatial_morton_sort(false)
            .with_spatial_hilbert_sort(false)
            .with_id_sort(false);
        Ok(Self {
            geometries,
            step,
            z,
            config,
        })
    }
}

fn coord(u: &mut Unstructured<'_>) -> arbitrary::Result<Coord<i32>> {
    Ok(Coord {
        x: u.int_in_range(-64..=4160)?,
        y: u.int_in_range(-64..=4160)?,
    })
}

fn line(u: &mut Unstructured<'_>, min: u8) -> arbitrary::Result<LineString<i32>> {
    let n = u.int_in_range(min..=8)?;
    Ok(LineString(
        (0..n).map(|_| coord(u)).collect::<arbitrary::Result<_>>()?,
    ))
}

fn ring(u: &mut Unstructured<'_>) -> arbitrary::Result<LineString<i32>> {
    let mut ring = line(u, 3)?;
    ring.close();
    Ok(ring)
}

fn polygon(u: &mut Unstructured<'_>) -> arbitrary::Result<Polygon<i32>> {
    let holes = u.int_in_range(0..=2u8)?;
    Ok(Polygon::new(
        ring(u)?,
        (0..holes)
            .map(|_| ring(u))
            .collect::<arbitrary::Result<_>>()?,
    ))
}

fn geometry(u: &mut Unstructured<'_>) -> arbitrary::Result<Geometry<i32>> {
    let parts = u.int_in_range(1..=3u8)?;
    Ok(match u.int_in_range(0..=5u8)? {
        0 => Geometry::Point(Point(coord(u)?)),
        1 => Geometry::LineString(line(u, 2)?),
        2 => Geometry::Polygon(polygon(u)?),
        3 => Geometry::MultiPoint(MultiPoint(
            (0..parts)
                .map(|_| Ok(Point(coord(u)?)))
                .collect::<arbitrary::Result<_>>()?,
        )),
        4 => Geometry::MultiLineString(MultiLineString(
            (0..parts)
                .map(|_| line(u, 2))
                .collect::<arbitrary::Result<_>>()?,
        )),
        _ => Geometry::MultiPolygon(MultiPolygon(
            (0..parts)
                .map(|_| polygon(u))
                .collect::<arbitrary::Result<_>>()?,
        )),
    })
}

impl ZInput {
    pub fn fuzz(self) {
        let cfg = self.config;
        let mut flat = TileLayer::new("z", 4096).expect("a valid layer");
        let mut with_z = TileLayer::new("z", 4096).expect("a valid layer");
        with_z.set_z_step(self.step).expect("an empty layer");
        for (geometry, z) in self.geometries.iter().zip(&self.z) {
            flat.push_feature(TileFeature::new(geometry.clone()))
                .expect("a flat feature");
            let mut feature = TileFeature::new(geometry.clone());
            feature.set_z(z.clone()).expect("one z per vertex");
            with_z.push_feature(feature).expect("a z feature");
        }

        let Some(flat) = encode(flat, cfg) else {
            return;
        };
        let decoded = encode(with_z, cfg).expect("z encodes wherever the flat layer does");
        assert_eq!(decoded.z_step(), Some(self.step));
        assert_eq!(strip_z(&decoded), flat, "z changed the decoded geometry");

        for (feature, (geometry, z)) in decoded
            .features()
            .iter()
            .zip(self.geometries.iter().zip(&self.z))
        {
            if feature.geometry() == geometry {
                assert_eq!(feature.z(), z.as_slice(), "z did not survive");
            }
            assert_z_follows_its_vertex(geometry, z, feature);
        }

        let staged = stage_tile(
            decoded.clone(),
            Unsorted,
            cfg.allow_shared_dict(),
            cfg.tessellate(),
        );
        let again = encode_decode(staged, cfg).expect("re-encoding a layer v2 produced");
        // Cutting stored triangles again may start each one at another corner.
        if cfg.tessellate() && cfg.allow_triangles_only() {
            for (before, after) in decoded.features().iter().zip(again.features()) {
                assert_z_follows_its_vertex(before.geometry(), before.z(), after);
            }
        } else {
            assert_eq!(again, decoded, "re-staging a z layer is not a fixpoint");
        }
    }
}

/// Every vertex of `after` that `geometry` holds at a single height carries that height.
fn assert_z_follows_its_vertex(geometry: &Geometry<i32>, z: &[i32], after: &TileFeature) {
    let mut heights: HashMap<Coord<i32>, Option<i32>> = HashMap::new();
    for (coord, &z) in stored_coords(geometry).into_iter().zip(z) {
        heights
            .entry(coord)
            .and_modify(|h| {
                if *h != Some(z) {
                    *h = None;
                }
            })
            .or_insert(Some(z));
    }
    for (coord, &z) in stored_coords(after.geometry()).into_iter().zip(after.z()) {
        if let Some(Some(height)) = heights.get(&coord) {
            assert_eq!(z, *height, "the vertex at {coord:?} lost its z");
        }
    }
}

/// The coordinates MLT stores for `geometry`, in the order its z run over them.
fn stored_coords(geometry: &Geometry<i32>) -> Vec<Coord<i32>> {
    let ring = |ring: &LineString<i32>| {
        let n = ring.0.len();
        let stored = if n > 1 && ring.0.first() == ring.0.last() {
            n - 1
        } else {
            n
        };
        ring.0[..stored].to_vec()
    };
    let polygon = |p: &Polygon<i32>| {
        std::iter::once(p.exterior())
            .chain(p.interiors())
            .flat_map(ring)
            .collect::<Vec<_>>()
    };
    match geometry {
        Geometry::Point(p) => vec![p.0],
        Geometry::LineString(ls) => ls.0.clone(),
        Geometry::Polygon(p) => polygon(p),
        Geometry::MultiPoint(mp) => mp.iter().map(|p| p.0).collect(),
        Geometry::MultiLineString(mls) => mls.iter().flat_map(|ls| ls.0.clone()).collect(),
        Geometry::MultiPolygon(mp) => mp.iter().flat_map(polygon).collect(),
        Geometry::Line(_)
        | Geometry::GeometryCollection(_)
        | Geometry::Rect(_)
        | Geometry::Triangle(_) => unreachable!("the input holds none"),
    }
}

/// Encode through the row model and decode back, or `None` when v2 has no layout for the layer.
fn encode(layer: TileLayer, cfg: EncoderConfig) -> Option<TileLayer> {
    match layer.encode(cfg) {
        Ok(bytes) => Some(decode(&bytes, 2)),
        Err(MltError::NotImplemented(_)) => None,
        Err(e) => panic!("encode should not fail: {e}"),
    }
}

fn strip_z(layer: &TileLayer) -> TileLayer {
    let mut flat = TileLayer::new(layer.name(), layer.extent().get()).expect("a valid layer");
    for feature in layer.features() {
        let mut copy = TileFeature::new(feature.geometry().clone());
        if let Some(id) = feature.id() {
            copy = TileFeature::with_id(feature.geometry().clone(), id);
        }
        flat.push_feature(copy).expect("a flat feature");
    }
    flat
}

impl std::fmt::Debug for ZInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ZInput {{\n\tconfig: {:#?}\n\tstep: {:?}\n\tgeometries: {:?}\n\tz: {:?}\n}}",
            self.config, self.step, self.geometries, self.z
        )
    }
}
