#![expect(dead_code)]

use mlt_core::geojson::FeatureCollection;
use mlt_core::v01::{DecodedGeometry, DecodedId, DecodedProperty, Encoder, GeometryType, IdEncoder, IdWidth, LogicalEncoder, OwnedGeometry, OwnedId, OwnedLayer01, OwnedProperty, PhysicalEncoder, PresenceStream, PropValue, PropertyEncoder};
use mlt_core::{Encodable as _, OwnedLayer, borrowme::Borrow as _};
use std::path::{Path, PathBuf};
use crate::geometry::{ValidatingGeometryEncoder};

#[derive(Debug, Clone)]
pub struct Feature {
    // features
    geom: DecodedGeometry,
    props: Vec<DecodedProperty>,
    ids: DecodedId,
    extent: Option<u32>,

    // config
    ids_encoder: IdEncoder,
    geometry_encoder: ValidatingGeometryEncoder,
    property_encoder: PropertyEncoder,
}

impl Feature {
    pub fn write(&self, dir: &Path, name: &str) {
        self.write_mlt(&dir.with_file_name(format!("{name}.mlt")));
        self.write_geojson(&dir.with_file_name(format!("{name}.geojson")));
    }
    pub fn point([x, y]: [i32; 2], meta: Encoder,vertex:Encoder) -> Self {
        let geom = DecodedGeometry {
            vector_types: vec![GeometryType::Point],
            vertices: Some(vec![x, y]),
            ..Default::default()
        };
        let mut geometry_encoder = ValidatingGeometryEncoder::default();
        geometry_encoder.point(meta,vertex);
        let property_encoder = PropertyEncoder::new(PresenceStream::Present, LogicalEncoder::None, PhysicalEncoder::None);

        Self {
            geom,
            props: vec![],
            ids: DecodedId(None),

            extent: None,
            ids_encoder: IdEncoder::new(LogicalEncoder::None, IdWidth::Id32),
            geometry_encoder,
            property_encoder,
        }
    }

    pub fn id(self, id: u64, ids_encoder: IdEncoder) -> Self {
        Self { ids: DecodedId(Some(vec![Some(id)])), ids_encoder,..self }
    }
    pub fn ids(self, ids: Vec<Option<u64>>, ids_encoder: IdEncoder) -> Self {
        Self { ids: DecodedId(Some(ids)), ids_encoder, ..self }
    }
    pub fn prop(self, name: &str, values: PropValue, property_encoder: PropertyEncoder) -> Self {
        let props = vec![DecodedProperty { name: name.to_string(), values }];
        Self { props, property_encoder, ..self }
    }
    pub fn props(self, props: Vec<DecodedProperty>, property_encoder: PropertyEncoder) -> Self {
        Self { props, property_encoder, ..self }
    }



    fn write_mlt(&self, file: &PathBuf) {
        let feat = self.clone();

        // encode to mlt
        let mut id = OwnedId::Decoded(feat.ids);
        id.encode_with(self.ids_encoder).unwrap();

        let mut geometry = OwnedGeometry::Decoded(feat.geom);
        geometry.encode_with(Box::new(self.geometry_encoder)).unwrap();

        let mut properties = feat.props.into_iter().map(OwnedProperty::Decoded).collect::<Vec<_>>();
        for p in &mut properties {
            p.encode_with(self.property_encoder).unwrap();
        }

        // serialise out
        let layer = OwnedLayer::Tag01(OwnedLayer01 {
            name: "layer1".to_string(),
            extent: self.extent.unwrap_or(4096),
            id,
            geometry,
            properties,
        });
        let mut buf = vec![0; 1024];
        layer.write_to(&mut buf).unwrap_or_else(|_| panic!("cannot encode feature {}", file.display()));

        // write to file
        std::fs::write(file, buf).unwrap_or_else(|_| panic!("cannot write feature {}", file.display()));
    }

    fn write_geojson(&self, file: &Path) {
        let feat = self.clone();

        let mut id = OwnedId::Decoded(feat.ids);
        id.encode_with(self.ids_encoder).unwrap();

        let mut geometry = OwnedGeometry::Decoded(feat.geom);
        geometry.encode_with(Box::new(self.geometry_encoder)).unwrap();

        let mut properties = feat.props.into_iter().map(OwnedProperty::Decoded).collect::<Vec<_>>();
        for p in &mut properties {
            p.encode_with(self.property_encoder).unwrap();
        }

        let layer = OwnedLayer::Tag01(OwnedLayer01 {
            name: "layer1".to_string(),
            extent: self.extent.unwrap_or(4096),
            id,
            geometry,
            properties,
        });
        let borrowed_layer = layer.borrow();

        let mlt_geojson = FeatureCollection::from_layers(&[borrowed_layer]).unwrap();
        let geojson = serde_json::to_string_pretty(&mlt_geojson).unwrap();
        std::fs::write(file, geojson).unwrap_or_else(|_| panic!("cannot write feature {}", file.display()));
    }

}
