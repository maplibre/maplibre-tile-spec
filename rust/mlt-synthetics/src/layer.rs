#![expect(dead_code)]

use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::{fs, io};

use mlt_core::geojson::FeatureCollection;
use mlt_core::v01::{
    DecodedGeometry, DecodedId, DecodedProperty, Encoder, GeometryType, IdEncoder, IdWidth,
    LogicalEncoder, OwnedGeometry, OwnedId, OwnedLayer01, OwnedProperty, PropValue,
    PropertyEncoder,
};
use mlt_core::{parse_layers, Encodable as _, OwnedLayer};

use crate::geometry::{Point, ValidatingGeometryEncoder};

#[derive(Debug, Clone)]
pub struct Feature {
    // features
    geom: DecodedGeometry,
    props: Vec<(DecodedProperty, PropertyEncoder)>,
    ids: DecodedId,
    extent: Option<u32>,

    // config
    ids_encoder: IdEncoder,
    geometry_encoder: ValidatingGeometryEncoder,
}

impl Feature {
    fn open_new(path: &Path) -> io::Result<File> {
        OpenOptions::new().write(true).create_new(true).open(path)
    }

    pub fn write(&self, dir: &Path, name: &str) {
        let path = dir.join(format!("{name}.mlt"));
        self.write_mlt(&path);

        let buffer = fs::read(&path).unwrap();
        let mut data = parse_layers(&buffer).unwrap();
        for layer in &mut data {
            layer.decode_all().unwrap();
        }
        let fc = FeatureCollection::from_layers(&data).unwrap();
        let json = serde_json::to_string_pretty(&serde_json::to_value(&fc).unwrap()).unwrap();
        let mut out_file = Self::open_new(&dir.join(format!("{name}.json"))).unwrap();
        out_file.write_all(json.as_bytes()).unwrap();
    }
    pub fn point(point: Point, meta: Encoder, vertex: Encoder) -> Self {
        default_feature().and_point(point, meta, vertex)
    }
    pub fn and_point(mut self, [x, y]: Point, meta: Encoder, vertex: Encoder) -> Self {
        self.geometry_encoder.point(meta, vertex);
        self.geom.vector_types.push(GeometryType::Point);
        let old_vert = self.geom.vertices.unwrap_or_default();
        let new_vert = vec![x, y];
        self.geom.vertices = Some(old_vert.into_iter().chain(new_vert).collect::<Vec<_>>());
        self
    }

    pub fn linestring(
        points: &[Point],
        meta: Encoder,
        vertex: Encoder,
        only_parts: Encoder,
    ) -> Self {
        default_feature().and_linestring(points, meta, vertex, only_parts)
    }

    pub fn and_linestring(
        mut self,
        points: &[Point],
        meta: Encoder,
        vertex: Encoder,
        only_parts: Encoder,
    ) -> Self {
        self.geometry_encoder.linestring(meta, vertex, only_parts);
        self.geom.vector_types.push(GeometryType::LineString);
        let old_vert = self.geom.vertices.unwrap_or_default();
        clet base_offset = old_vert.len() as u32;
        let new_vert = points.into_iter().copied().flatten().collect::<Vec<_>>();
        let new_offset = base_offset + (new_vert.len() as u32) / 2;

        self.geom.vertices = Some(
            old_vert
                .into_iter()
                .chain(new_vert.into_iter())
                .collect::<Vec<_>>(),
        );

        let new_part_offsets = vec![base_offset, new_offset];
        let old_part_offsets = self.geom.part_offsets.unwrap_or_default();
        self.geom.part_offsets = Some(
            old_part_offsets
                .into_iter()
                .chain(new_part_offsets)
                .collect::<Vec<_>>(),
        );

        self
    }

    pub fn polygon(points: &[Point]) -> Self {
        default_feature().and_polygon(points)
    }

    pub fn and_polygon(mut self, points: &[Point]) -> Self {
        todo!()
    }

    pub fn polygon_with_hole(points: &[Point], hole: &[Point]) -> Self {
        default_feature().and_polygon_with_hole(points, hole)
    }
    pub fn and_polygon_with_hole(mut self, points: &[Point], hole: &[Point]) -> Self {
        todo!()
    }
    pub fn multi_point(points: &[Point]) -> Self {
        default_feature().and_multi_point(points)
    }
    pub fn and_multi_point(mut self,points: &[Point]) -> Self {
        todo!()
    }
    pub fn multi_linestring(points: &[&[Point]]) -> Self {
        default_feature().and_multi_linestring(points)
    }
    pub fn and_multi_linestring(mut self, points: &[&[Point]]) -> Self {
        todo!()
    }
    pub fn multi_polygon(points: &[&[Point]]) -> Self {
        default_feature().and_multi_polygon(points)
    }
    pub fn and_multi_polygon(mut self, points: &[&[Point]]) -> Self {
        todo!()
    }

    pub fn id(self, id: u64, logical: LogicalEncoder, id_width: IdWidth) -> Self {
        let ids_encoder = IdEncoder::new(logical, id_width);
        Self {
            ids: DecodedId(Some(vec![Some(id)])),
            ids_encoder,
            ..self
        }
    }
    pub fn ids(self, ids: Vec<Option<u64>>, ids_encoder: IdEncoder) -> Self {
        let ids = DecodedId(Some(ids));
        Self {
            ids,
            ids_encoder,
            ..self
        }
    }
    pub fn prop(self, name: impl ToString, values: PropValue, encoder: PropertyEncoder) -> Self {
        let name = name.to_string();
        Self {
            props: vec![(DecodedProperty { name, values }, encoder)],
            ..self
        }
    }
    pub fn props(self, props: Vec<DecodedProperty>, encoder: PropertyEncoder) -> Self {
        Self {
            props: props.into_iter().map(|p| (p, encoder)).collect(),
            ..self
        }
    }

    pub fn extent(self, extent: u32) -> Self {
        Self {
            extent: Some(extent),
            ..self
        }
    }

    fn write_mlt(&self, path: &PathBuf) {
        let feat = self.clone();

        // encode to mlt
        let mut geometry = OwnedGeometry::Decoded(feat.geom);
        geometry
            .encode_with(Box::new(self.geometry_encoder))
            .unwrap();

        // serialize as binary
        let layer = OwnedLayer::Tag01(OwnedLayer01 {
            name: "layer1".to_string(),
            extent: self.extent.unwrap_or(4096),
            id: if self.ids.0.is_some() {
                let mut id = OwnedId::Decoded(feat.ids);
                id.encode_with(self.ids_encoder).unwrap();
                id
            } else {
                OwnedId::None
            },
            geometry,
            properties: feat
                .props
                .into_iter()
                .map(|(p, e)| {
                    let mut p = OwnedProperty::Decoded(p);
                    p.encode_with(e).unwrap();
                    p
                })
                .collect::<Vec<_>>(),
        });

        let mut file = Self::open_new(path)
            .unwrap_or_else(|_| panic!("cannot create feature {}", path.display()));
        layer
            .write_to(&mut file)
            .unwrap_or_else(|_| panic!("cannot encode feature {}", path.display()));
    }
}

// purposely not pub, or impl Default since it is REQUIRED to have at least one geometry
fn default_feature() -> Feature {
    Feature {
        geom: DecodedGeometry::default(),
        props: vec![],
        ids: DecodedId(None),

        extent: None,
        ids_encoder: IdEncoder::new(LogicalEncoder::None, IdWidth::Id32),
        geometry_encoder: ValidatingGeometryEncoder::default(),
    }
}
