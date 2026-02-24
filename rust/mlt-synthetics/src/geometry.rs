#![expect(dead_code)]

use mlt_core::v01::{Encoder, GeometryEncoder};

pub type Point = [i32; 2];
pub const C0: Point = [13, 42];
pub const C1: Point = [4, 47];
pub const C2: Point = [12, 53];
pub const C3: Point = [18, 45];
pub const H1: Point = [13, 48];
pub const H2: Point = [12, 50];
pub const H3: Point = [10, 49];

#[derive(Default, Clone, Copy, Debug)]
pub struct ValidatingGeometryEncoder {
    pub meta: Option<Encoder>,
    pub num_geometries: Option<Encoder>,
    pub rings: Option<Encoder>,
    pub rings2: Option<Encoder>,
    pub no_rings: Option<Encoder>,
    pub parts: Option<Encoder>,
    pub parts_ring: Option<Encoder>,
    pub only_parts: Option<Encoder>,
    pub triangles: Option<Encoder>,
    pub triangles_indexes: Option<Encoder>,
    pub vertex: Option<Encoder>,
    pub vertex_offsets: Option<Encoder>,
}
impl ValidatingGeometryEncoder {
    /// Configure encoding for Point geometry.
    pub fn point(mut self, meta: Encoder, vertex: Encoder) -> Self {
        set(&mut self.meta, meta, "meta");
        set(&mut self.vertex, vertex, "vertex");
        self
    }

    /// Configure encoding for `LineString` geometry.
    pub fn linestring(mut self, meta: Encoder, vertex: Encoder, only_parts: Encoder) -> Self {
        set(&mut self.meta, meta, "meta");
        set(&mut self.vertex, vertex, "vertex");
        set(&mut self.only_parts, only_parts, "only_parts");
        self
    }

    /// Configure encoding for Polygon geometry.
    pub fn polygon(
        mut self,
        meta: Encoder,
        vertex: Encoder,
        parts: Encoder,
        parts_ring: Encoder,
    ) -> Self {
        set(&mut self.meta, meta, "meta");
        set(&mut self.vertex, vertex, "vertex");
        set(&mut self.parts, parts, "parts");
        set(&mut self.parts_ring, parts_ring, "parts_ring");
        self
    }

    /// Configure encoding for Polygon geometry with tessellation.
    /// This includes the standard polygon streams plus triangles and index buffer.
    #[expect(clippy::too_many_arguments)]
    pub fn polygon_tessellated(
        mut self,
        meta: Encoder,
        vertex: Encoder,
        num_geometries: Encoder,
        parts: Encoder,
        parts_ring: Encoder,
        triangles: Encoder,
        triangles_indexes: Encoder,
    ) -> Self {
        set(&mut self.meta, meta, "meta");
        set(&mut self.vertex, vertex, "vertex");
        set(&mut self.num_geometries, num_geometries, "num_geometries");
        set(&mut self.parts, parts, "parts");
        set(&mut self.parts_ring, parts_ring, "parts_ring");
        set(&mut self.triangles, triangles, "triangles");
        set(
            &mut self.triangles_indexes,
            triangles_indexes,
            "triangles_indexes",
        );
        self
    }

    /// Configure encoding for `MultiPoint` geometry.
    pub fn multi_point(mut self, meta: Encoder, vertex: Encoder, num_geometries: Encoder) -> Self {
        set(&mut self.meta, meta, "meta");
        set(&mut self.vertex, vertex, "vertex");
        set(&mut self.num_geometries, num_geometries, "num_geometries");
        self
    }

    /// Configure encoding for `MultiLineString` geometry.
    /// Uses `no_rings` since `MultiLineString` has `geometry_offsets` but no rings.
    pub fn multi_linestring(
        mut self,
        meta: Encoder,
        vertex: Encoder,
        num_geometries: Encoder,
        no_rings: Encoder,
    ) -> Self {
        set(&mut self.meta, meta, "meta");
        set(&mut self.vertex, vertex, "vertex");
        set(&mut self.num_geometries, num_geometries, "num_geometries");
        set(&mut self.no_rings, no_rings, "no_rings");
        self
    }

    /// Configure encoding for `MultiPolygon` geometry.
    /// Uses `rings` and `rings2` since `MultiPolygon` has `geometry_offsets`.
    pub fn multi_polygon(
        mut self,
        meta: Encoder,
        vertex: Encoder,
        num_geometries: Encoder,
        rings: Encoder,
        rings2: Encoder,
    ) -> Self {
        set(&mut self.meta, meta, "meta");
        set(&mut self.vertex, vertex, "vertex");
        set(&mut self.num_geometries, num_geometries, "num_geometries");
        set(&mut self.rings, rings, "rings");
        set(&mut self.rings2, rings2, "rings2");
        self
    }

    /// Merge another encoder's settings into this one.
    /// Panics if any field is set differently in both encoders.
    pub fn merge(mut self, other: Self) -> Self {
        merge_opt(&mut self.meta, other.meta, "meta");
        merge_opt(
            &mut self.num_geometries,
            other.num_geometries,
            "num_geometries",
        );
        merge_opt(&mut self.rings, other.rings, "rings");
        merge_opt(&mut self.rings2, other.rings2, "rings2");
        merge_opt(&mut self.no_rings, other.no_rings, "no_rings");
        merge_opt(&mut self.parts, other.parts, "parts");
        merge_opt(&mut self.parts_ring, other.parts_ring, "parts_ring");
        merge_opt(&mut self.only_parts, other.only_parts, "only_parts");
        merge_opt(&mut self.triangles, other.triangles, "triangles");
        merge_opt(
            &mut self.triangles_indexes,
            other.triangles_indexes,
            "triangles_indexes",
        );
        merge_opt(&mut self.vertex, other.vertex, "vertex");
        merge_opt(
            &mut self.vertex_offsets,
            other.vertex_offsets,
            "vertex_offsets",
        );
        self
    }
}

impl GeometryEncoder for ValidatingGeometryEncoder {
    fn meta(&self) -> Encoder {
        self.meta.expect("meta")
    }
    fn num_geometries(&self) -> Encoder {
        self.num_geometries.expect("num_geometries")
    }
    fn rings(&self) -> Encoder {
        self.rings.expect("rings")
    }
    fn rings2(&self) -> Encoder {
        self.rings2.expect("rings2")
    }
    fn no_rings(&self) -> Encoder {
        self.no_rings.expect("no_rings")
    }
    fn parts(&self) -> Encoder {
        self.parts.expect("parts")
    }
    fn parts_ring(&self) -> Encoder {
        self.parts_ring.expect("parts_ring")
    }
    fn only_parts(&self) -> Encoder {
        self.only_parts.expect("only_parts")
    }
    fn triangles(&self) -> Encoder {
        self.triangles.expect("triangles")
    }
    fn triangles_indexes(&self) -> Encoder {
        self.triangles_indexes.expect("triangles_indexes")
    }
    fn vertex(&self) -> Encoder {
        self.vertex.expect("vertex")
    }
    fn vertex_offsets(&self) -> Encoder {
        self.vertex_offsets.expect("vertex_offsets")
    }
}

fn set(val: &mut Option<Encoder>, encoder: Encoder, name: &str) {
    if let Some(v) = val
        && v != &encoder
    {
        panic!("{name} already set with different value")
    }
    *val = Some(encoder);
}

fn merge_opt(target: &mut Option<Encoder>, source: Option<Encoder>, name: &str) {
    if let Some(src) = source {
        if let Some(tgt) = target {
            assert_eq!(tgt, &src, "{name} conflict during merge");
        } else {
            *target = Some(src);
        }
    }
}
