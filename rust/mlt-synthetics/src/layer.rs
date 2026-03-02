#![expect(dead_code)]

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::{fs, io};

use geo::{Convert as _, TriangulateEarcut as _};
use geo_types::{LineString, Polygon};
use mlt_core::geojson::{FeatureCollection, Geom32};
use mlt_core::v01::PropValue::{Bool, F32, F64, I32, I64, Str, U32, U64};
use mlt_core::v01::{
    DecodedGeometry, DecodedId, DecodedProperty, GeometryEncoder, IdEncoder, IntegerEncoder,
    MultiPropertyEncoder, OwnedEncodedProperty, OwnedGeometry, OwnedId, OwnedLayer01,
    OwnedProperty, PresenceStream, PropValue, PropertyEncoder, ScalarEncoder, StringEncoding,
    VertexBufferType,
};
use mlt_core::{Encodable as _, FromDecoded as _, OwnedLayer, parse_layers};

/// Tessellate a polygon using the geo crate's earcut algorithm.
///
/// Geo's earcut includes the closing vertex in each ring; MLT (and Java's `earcut4j`) omit it.
/// We remap triangle indices so that any index referring to a ring's closing vertex is replaced
/// by that ring's start index, producing identical index buffers to Java.
fn tessellate_polygon(polygon: &Polygon<i32>) -> (Vec<u32>, u32) {
    // Convert i32 polygon to f64 for tessellation (geo's TriangulateEarcut requires CoordFloat)
    let polygon_f64: Polygon<f64> = polygon.convert();
    let raw = polygon_f64.earcut_triangles_raw();
    let num_triangles = u32::try_from(raw.triangle_indices.len() / 3).expect("too many triangles");

    // Build remap: geo index -> MLT index (closing vertex of each ring -> ring start).
    let mut geo_to_mlt = Vec::with_capacity(raw.vertices.len() / 2);
    let mut mlt_offset = 0;

    let mut push_ring = |ring: &LineString<i32>| {
        let len = ring.0.len();
        let mlt_len = if len > 1 && ring.0.first() == ring.0.last() {
            len - 1
        } else {
            len
        };
        for i in 0..len {
            geo_to_mlt.push(if i == len - 1 && mlt_len < len {
                mlt_offset
            } else {
                mlt_offset + i
            });
        }
        mlt_offset += mlt_len;
    };

    push_ring(polygon.exterior());
    for interior in polygon.interiors() {
        push_ring(interior);
    }

    let indices_u32: Vec<u32> = raw
        .triangle_indices
        .into_iter()
        .map(|i| {
            let mlt_idx = geo_to_mlt.get(i).copied().unwrap_or(i);
            u32::try_from(mlt_idx).expect("index overflow")
        })
        .collect();

    (indices_u32, num_triangles)
}

pub struct SynthWriter {
    dir: PathBuf,
}

impl SynthWriter {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    #[must_use]
    pub fn geo(&self, encoder: IntegerEncoder) -> Layer {
        Layer::new(self.dir.clone(), encoder)
    }

    /// Create a layer with all geometry encoders set to `VarInt`.
    #[must_use]
    pub fn geo_varint(&self) -> Layer {
        Layer::new(self.dir.clone(), IntegerEncoder::varint())
    }

    /// Create a layer with all geometry encoders set to `FastPFOR`.
    #[must_use]
    pub fn geo_fastpfor(&self) -> Layer {
        Layer::new(self.dir.clone(), IntegerEncoder::fastpfor())
    }
}

/// Layer builder: holds geometry encoder, geometry list, properties, extent, and IDs.
pub struct Layer {
    pub path: PathBuf,
    pub geometry_encoder: GeometryEncoder,
    pub geometry_items: Vec<Geom32>,
    /// Polygons that are also tessellated; triangle data is merged when building decoded geometry.
    pub tessellated_polygons: Vec<Option<Polygon<i32>>>,
    pub props: Vec<Box<dyn LayerProp>>,
    pub extent: Option<u32>,
    pub ids: Option<(Vec<Option<u64>>, IdEncoder)>,
    pub shared_dicts: HashMap<String, StringEncoding>,
}

impl Layer {
    #[must_use]
    pub fn new(path: PathBuf, default_geom_enc: IntegerEncoder) -> Layer {
        Layer {
            path,
            geometry_encoder: GeometryEncoder::all(default_geom_enc),
            geometry_items: vec![],
            tessellated_polygons: vec![],
            props: vec![],
            extent: None,
            ids: None,
            shared_dicts: HashMap::new(),
        }
    }

    /// Set encoding for parts length stream when rings are present.
    #[must_use]
    pub fn rings(mut self, e: IntegerEncoder) -> Self {
        self.geometry_encoder.rings(e);
        self
    }

    /// Set encoding for ring vertex-count stream.
    #[must_use]
    pub fn rings2(mut self, e: IntegerEncoder) -> Self {
        self.geometry_encoder.rings2(e);
        self
    }

    /// Set encoding for the vertex data stream.
    #[must_use]
    pub fn vertex(mut self, e: IntegerEncoder) -> Self {
        self.geometry_encoder.vertex(e);
        self
    }

    /// Set encoding for the geometry types (meta) stream.
    #[must_use]
    pub fn meta(mut self, e: IntegerEncoder) -> Self {
        self.geometry_encoder.meta(e);
        self
    }

    /// Set encoding for the geometry length stream.
    #[must_use]
    pub fn geometries(mut self, e: IntegerEncoder) -> Self {
        self.geometry_encoder.geometries(e);
        self
    }

    /// Set encoding for parts length stream when rings are not present.
    #[must_use]
    pub fn no_rings(mut self, e: IntegerEncoder) -> Self {
        self.geometry_encoder.no_rings(e);
        self
    }

    /// Set encoding for parts length stream (with rings) when `geometry_offsets` absent.
    #[must_use]
    pub fn parts(mut self, e: IntegerEncoder) -> Self {
        self.geometry_encoder.parts(e);
        self
    }

    /// Set encoding for ring lengths when `geometry_offsets` absent.
    #[must_use]
    pub fn parts_ring(mut self, e: IntegerEncoder) -> Self {
        self.geometry_encoder.parts_ring(e);
        self
    }

    /// Set encoding for parts-only stream.
    #[must_use]
    pub fn only_parts(mut self, e: IntegerEncoder) -> Self {
        self.geometry_encoder.only_parts(e);
        self
    }

    /// Set encoding for triangles and triangle index buffer.
    #[must_use]
    pub fn triangles(mut self, e: IntegerEncoder) -> Self {
        self.geometry_encoder.triangles(e);
        self.geometry_encoder.triangles_indexes(e);
        self
    }

    /// Set encoding for vertex offsets.
    #[must_use]
    pub fn vertex_offsets(mut self, e: IntegerEncoder) -> Self {
        self.geometry_encoder.vertex_offsets(e);
        self
    }

    /// Set encoding of the vertex buffer.
    #[must_use]
    pub fn vertex_buffer_type(mut self, v: VertexBufferType) -> Self {
        self.geometry_encoder.vertex_buffer_type(v);
        self
    }

    /// Add a geometry (uses [`geo_types::Geometry`] `From` impls: `Point`, `LineString`, etc.).
    #[must_use]
    pub fn geo(mut self, geometry: impl Into<Geom32>) -> Self {
        self.geometry_items.push(geometry.into());
        self.tessellated_polygons.push(None);
        self
    }

    /// Add multiple geometries
    #[must_use]
    pub fn geos<T: Into<Geom32>, I: IntoIterator<Item = T>>(mut self, geometries: I) -> Self {
        for g in geometries {
            self = self.geo(g.into());
        }
        self
    }

    /// Add a tessellated polygon (polygon + triangle mesh).
    #[must_use]
    pub fn tessellated(mut self, polygon: Polygon<i32>) -> Self {
        self.geometry_items.push(Geom32::Polygon(polygon.clone()));
        self.tessellated_polygons.push(Some(polygon));
        self
    }

    /// Add a property (boxed dynamic value).
    #[must_use]
    pub fn add_prop(
        mut self,
        encoder: ScalarEncoder,
        name: impl Into<String>,
        prop: PropValue,
    ) -> Self {
        let prop = DecodedProperty {
            name: name.into(),
            values: prop,
        };

        self.props.push(Box::new(DecodedProp::new(prop, encoder)));
        self
    }

    /// Register a shared dictionary.
    ///
    /// All children with the same `struct_name` are grouped into one struct column. Children are
    /// ordered within the struct by the order they are added. The struct column is sorted among
    /// other columns by `struct_name`.
    ///
    /// See [`Self::add_shared_dict_column`] to add children.
    #[must_use]
    pub fn add_shared_dict(
        mut self,
        struct_name: impl Into<String>,
        encoder: StringEncoding,
    ) -> Self {
        self.shared_dicts.insert(struct_name.into(), encoder);
        self
    }

    /// Add a child field of a shared-dictionary struct column.
    ///
    /// All children with the same `struct_name` are grouped into one struct column. Children are
    /// ordered within the struct by the order they are added. The struct column is sorted among
    /// other columns by `struct_name`.
    ///
    /// Needs [`Self::add_shared_dict`] called before adding columns.
    #[must_use]
    pub fn add_shared_dict_column(
        mut self,
        struct_name: &str,
        child_name: &str,
        optional: PresenceStream,
        offset: IntegerEncoder,
        values: impl IntoIterator<Item = Option<String>>,
    ) -> Self {
        assert!(
            self.shared_dicts.contains_key(struct_name),
            "shared dictionary needs to be registered before adding columns"
        );
        self.props.push(Box::new(StructChild {
            struct_name: struct_name.to_string(),
            child_name: child_name.to_string(),
            values: values.into_iter().collect(),
            optional,
            offset,
        }));
        self
    }

    /// Set the tile extent.
    #[must_use]
    pub fn extent(mut self, extent: u32) -> Self {
        self.extent = Some(extent);
        self
    }

    /// Set feature IDs.
    #[must_use]
    pub fn ids(mut self, ids: Vec<Option<u64>>, encoder: IdEncoder) -> Self {
        self.ids = Some((ids, encoder));
        self
    }

    fn build_decoded_geometry(&self) -> DecodedGeometry {
        let mut geom = DecodedGeometry::default();
        for g in &self.geometry_items {
            geom.push_geom(g);
        }
        for poly in &self.tessellated_polygons {
            let Some(poly) = poly else { continue };
            let (indices, num_triangles) = tessellate_polygon(poly);
            geom.triangles
                .get_or_insert_with(Vec::new)
                .push(num_triangles);
            geom.index_buffer
                .get_or_insert_with(Vec::new)
                .extend(indices);
        }
        geom
    }

    fn open_new(path: &Path) -> io::Result<File> {
        OpenOptions::new().write(true).create_new(true).open(path)
    }

    /// Write the layer to an MLT file and a corresponding JSON file (consumes self).
    pub fn write(self, name: impl AsRef<str>) {
        let name = name.as_ref();
        let dir = self.path.clone();
        let path = dir.join(format!("{name}.mlt"));
        self.write_mlt(&path);

        let buffer = fs::read(&path).unwrap();
        let mut data = parse_layers(&buffer).unwrap();
        for l in &mut data {
            l.decode_all().unwrap();
        }
        let fc = FeatureCollection::from_layers(&data).unwrap();
        let mut json = serde_json::to_string_pretty(&fc).unwrap();
        json.push('\n');
        let mut out_file = Self::open_new(&dir.join(format!("{name}.json"))).unwrap();
        out_file.write_all(json.as_bytes()).unwrap();
    }

    fn write_mlt(self, path: &Path) {
        let decoded_geom = self.build_decoded_geometry();
        let mut geometry = OwnedGeometry::Decoded(decoded_geom);
        geometry.encode_with(self.geometry_encoder).unwrap();

        let mut all_props = self
            .props
            .iter()
            .map(|p| p.to_decoded())
            .collect::<Vec<_>>();

        // Sort by effective column name: struct_name for struct children, property name for
        // scalars. sort_by is stable so children of the same struct keep their relative order.
        all_props.sort_by(|(pa, ia), (pb, ib)| {
            effective_column_name(pa, ia).cmp(effective_column_name(pb, ib))
        });

        let (decoded, instructions): (Vec<_>, Vec<_>) = all_props.into_iter().unzip();
        let enc = MultiPropertyEncoder::new(instructions, self.shared_dicts);
        let encoded_props = Vec::<OwnedEncodedProperty>::from_decoded(&decoded, enc).unwrap();

        let id = if let Some((ids, ids_encoder)) = self.ids {
            let mut id = OwnedId::Decoded(DecodedId(Some(ids)));
            id.encode_with(ids_encoder).unwrap();
            id
        } else {
            OwnedId::None
        };

        let properties = encoded_props
            .into_iter()
            .map(OwnedProperty::Encoded)
            .collect();
        let layer = OwnedLayer::Tag01(OwnedLayer01 {
            name: "layer1".to_string(),
            extent: self.extent.unwrap_or(80),
            id,
            geometry,
            properties,
        });

        let mut file = Self::open_new(path)
            .unwrap_or_else(|e| panic!("cannot create {}: {e}", path.display()));
        layer
            .write_to(&mut file)
            .unwrap_or_else(|e| panic!("cannot encode {}: {e}", path.display()));
    }
}

/// Returns the effective column name used for sorting: `struct_name` for struct children,
/// `prop.name` for scalars.
fn effective_column_name<'a>(prop: &'a DecodedProperty, encoder: &'a PropertyEncoder) -> &'a str {
    match encoder {
        PropertyEncoder::Scalar(_) => &prop.name,
        PropertyEncoder::SharedDict(enc) => &enc.struct_name,
    }
}

/// Property builder that can be added to a layer as a boxed dynamic value.
pub trait LayerProp {
    fn to_decoded(&self) -> (DecodedProperty, PropertyEncoder);
}

/// Dynamic accessor: pushes an optional value onto the property's value list.
/// Stored as a boxed closure so we can have a uniform Prop<T> API.
type SetValue<T> = Box<dyn FnMut(&mut Vec<Option<T>>, Option<T>)>;

/// Property builder for a single property with typed values.
pub struct Prop<T> {
    name: String,
    enc: ScalarEncoder,
    values: Vec<Option<T>>,
    set_value: SetValue<T>,
}

impl<T: Clone> Prop<T> {
    pub fn new(name: &str, enc: ScalarEncoder, set_value: SetValue<T>) -> Self {
        Self {
            name: name.to_string(),
            enc,
            values: vec![],
            set_value,
        }
    }

    /// Add an optional value.
    #[must_use]
    pub fn add_none(mut self) -> Self {
        (self.set_value)(&mut self.values, None);
        self
    }

    #[must_use]
    pub fn add(mut self, value: T) -> Self {
        (self.set_value)(&mut self.values, Some(value));
        self
    }

    fn to_decoded_with(&self, values: PropValue) -> (DecodedProperty, PropertyEncoder) {
        (
            DecodedProperty {
                name: self.name.clone(),
                values,
            },
            PropertyEncoder::Scalar(self.enc),
        )
    }
}

impl LayerProp for Prop<bool> {
    fn to_decoded(&self) -> (DecodedProperty, PropertyEncoder) {
        self.to_decoded_with(Bool(self.values.clone()))
    }
}
impl LayerProp for Prop<i32> {
    fn to_decoded(&self) -> (DecodedProperty, PropertyEncoder) {
        self.to_decoded_with(I32(self.values.clone()))
    }
}
impl LayerProp for Prop<u32> {
    fn to_decoded(&self) -> (DecodedProperty, PropertyEncoder) {
        self.to_decoded_with(U32(self.values.clone()))
    }
}
impl LayerProp for Prop<i64> {
    fn to_decoded(&self) -> (DecodedProperty, PropertyEncoder) {
        self.to_decoded_with(I64(self.values.clone()))
    }
}
impl LayerProp for Prop<u64> {
    fn to_decoded(&self) -> (DecodedProperty, PropertyEncoder) {
        self.to_decoded_with(U64(self.values.clone()))
    }
}
impl LayerProp for Prop<f32> {
    fn to_decoded(&self) -> (DecodedProperty, PropertyEncoder) {
        self.to_decoded_with(F32(self.values.clone()))
    }
}
impl LayerProp for Prop<f64> {
    fn to_decoded(&self) -> (DecodedProperty, PropertyEncoder) {
        self.to_decoded_with(F64(self.values.clone()))
    }
}
impl LayerProp for Prop<String> {
    fn to_decoded(&self) -> (DecodedProperty, PropertyEncoder) {
        self.to_decoded_with(Str(self.values.clone()))
    }
}

/// Push closure: appends to the vec. Used as the dynamic accessor for all Prop<T>.
fn push_value<T>(v: &mut Vec<Option<T>>, x: Option<T>) {
    v.push(x);
}

pub fn bool(name: &str, enc: ScalarEncoder) -> Prop<bool> {
    Prop::new(name, enc, Box::new(push_value))
}

pub fn i32(name: &str, enc: ScalarEncoder) -> Prop<i32> {
    Prop::new(name, enc, Box::new(push_value))
}

pub fn u32(name: &str, enc: ScalarEncoder) -> Prop<u32> {
    Prop::new(name, enc, Box::new(push_value))
}

pub fn i64(name: &str, enc: ScalarEncoder) -> Prop<i64> {
    Prop::new(name, enc, Box::new(push_value))
}

pub fn u64(name: &str, enc: ScalarEncoder) -> Prop<u64> {
    Prop::new(name, enc, Box::new(push_value))
}

pub fn f32(name: &str, enc: ScalarEncoder) -> Prop<f32> {
    Prop::new(name, enc, Box::new(push_value))
}

pub fn f64(name: &str, enc: ScalarEncoder) -> Prop<f64> {
    Prop::new(name, enc, Box::new(push_value))
}

pub fn string(name: &str, enc: ScalarEncoder) -> Prop<String> {
    Prop::new(name, enc, Box::new(push_value))
}

/// Erased property: holds a pre-built decoded property and encoder (e.g. for I32, Str, etc.).
#[derive(Clone)]
pub struct DecodedProp {
    prop: DecodedProperty,
    enc: ScalarEncoder,
}

impl DecodedProp {
    #[must_use]
    pub fn new(prop: DecodedProperty, enc: ScalarEncoder) -> Self {
        Self { prop, enc }
    }
}
impl LayerProp for DecodedProp {
    fn to_decoded(&self) -> (DecodedProperty, PropertyEncoder) {
        (self.prop.clone(), PropertyEncoder::Scalar(self.enc))
    }
}

/// A single child field of a shared-dictionary struct column.
///
/// All `StructChildProp`s added to a [`Layer`] with the same `struct_name` are grouped into one
/// struct column. The column appears in the output at the position of its first child after
/// sorting by effective column name.
pub struct StructChild {
    struct_name: String,
    child_name: String,
    values: Vec<Option<String>>,
    optional: PresenceStream,
    offset: IntegerEncoder,
}

impl LayerProp for StructChild {
    fn to_decoded(&self) -> (DecodedProperty, PropertyEncoder) {
        let prop = DecodedProperty {
            name: self.child_name.clone(),
            values: Str(self.values.clone()),
        };
        let instruction = PropertyEncoder::shared_dict(
            &self.struct_name,
            &self.child_name,
            self.optional,
            self.offset,
        );
        (prop, instruction)
    }
}
