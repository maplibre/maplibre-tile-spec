#![expect(dead_code)]

use mlt_core::v01::{Encoder, GeometryEncoder};

pub const C0: [i32; 2] = [13, 42];
pub const C1: [i32; 2] = [4, 47];
pub const C2: [i32; 2] = [12, 53];
pub const C3: [i32; 2] = [18, 45];
pub const H1: [i32; 2] = [13, 48];
pub const H2: [i32; 2] = [12, 50];
pub const H3: [i32; 2] = [10, 49];

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
    pub fn point(&mut self, meta: Encoder, vertex: Encoder) {
        set(&mut self.meta, meta, "meta");
        set(&mut self.vertex, vertex, "vertex");
    }
    pub fn polyline(&mut self, meta: Encoder, rings: Encoder, rings2: Encoder, no_rings: Encoder) {
        set(&mut self.meta, meta, "meta");
        set(&mut self.rings, rings, "rings");
        set(&mut self.rings2, rings2, "rings2");
        set(&mut self.no_rings, no_rings, "no_rings");
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
        panic!("{name} already set")
    }
    *val = Some(encoder);
}
