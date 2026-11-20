//! A layer written one feature at a time into columns, then encoded without building a
//! [`TileLayer`](crate::TileLayer) or any value per feature.

use std::ops::Range;

use bitvec::vec::BitVec;
#[cfg(feature = "unstable-v2")]
use compact_str::CompactString;
use fast_mvt::{MvtFeatureBuilder, MvtGeomType, MvtLayerBuilder, MvtTileBuilder};
use geo_types::Coord;

use crate::decoder::{GeometryType, GeometryValues};
use crate::encoder::{EncoderConfig, LayerSource, encode_layer};
#[cfg(feature = "unstable-v2")]
use crate::encoder::{StagedValues, extend_run, wound_polygons_order};
use crate::mvt::write_mvt_layer;
use crate::tile::{
    ColumnRole, Extent, PropKind, PropertyKey, reject_taken_name, validate_layer_name,
};
#[cfg(feature = "unstable-v2")]
use crate::tile::{MValueKey, stored_len};
use crate::{MltError, MltResult, PropValueRef};

/// A layer written feature by feature, straight from borrowed values, and encoded exactly as the
/// same features in a [`TileLayer`](crate::TileLayer) would be.
///
/// Names and string values are borrowed for `'a`.
/// [`LayerWriter::reset`] keeps the buffers for the next layer.
/// A [`PropertyKey`] or `MValueKey` means nothing after [`LayerWriter::reset`].
///
/// ```
/// use mlt_core::encoder::{EncoderConfig, Network, Profile};
/// use mlt_core::geo_types::Coord;
/// use mlt_core::{GeometryType, LayerWriter, PropKind};
///
/// let mut layer = LayerWriter::new("pois", 4096)?;
/// let name = layer.add_property("name", PropKind::Str)?;
/// let mut feature = layer.feature(GeometryType::Point);
/// feature.id(Some(1)).points([Coord { x: 10, y: 20 }])?.property(name, "Cafe")?;
/// feature.finish()?;
/// let mlt = layer.encode(EncoderConfig::from_profile(Profile::Native(Network::Cellular4G)))?;
/// # Ok::<(), mlt_core::MltError>(())
/// ```
#[derive(Debug, Clone)]
pub struct LayerWriter<'a> {
    name: &'a str,
    extent: Extent,
    property_names: Vec<&'a str>,
    property_kinds: Vec<PropKind>,
    columns: Vec<Column<'a>>,
    /// Columns of an earlier layer, kept for their buffers.
    spare_columns: Vec<Column<'a>>,
    ids: Vec<Option<u64>>,
    geometry: Geometries,
    #[cfg(feature = "unstable-v2")]
    m_values: MValueColumns<'a>,
}

/// One value or none per feature. Strings are kept as references, and every other kind as the
/// bits of its value.
#[derive(Debug, Clone)]
struct Column<'a> {
    kind: PropKind,
    values: ColumnValues<'a>,
    present: BitVec<u8>,
}

#[derive(Debug, Clone)]
enum ColumnValues<'a> {
    Scalars(Vec<u64>),
    Strs(Vec<&'a str>),
}

impl ColumnValues<'_> {
    fn new(kind: PropKind) -> Self {
        if kind == PropKind::Str {
            Self::Strs(Vec::new())
        } else {
            Self::Scalars(Vec::new())
        }
    }

    /// Empties the values and makes them hold values of `kind`.
    fn restart(&mut self, kind: PropKind) {
        self.truncate(0);
        if (kind == PropKind::Str) != matches!(self, Self::Strs(_)) {
            *self = Self::new(kind);
        }
    }

    #[cfg(feature = "unstable-v2")]
    fn len(&self) -> usize {
        match self {
            Self::Scalars(bits) => bits.len(),
            Self::Strs(strs) => strs.len(),
        }
    }

    fn truncate(&mut self, len: usize) {
        match self {
            Self::Scalars(bits) => bits.truncate(len),
            Self::Strs(strs) => strs.truncate(len),
        }
    }
}

impl<'a> Column<'a> {
    fn new(kind: PropKind) -> Self {
        Self {
            kind,
            values: ColumnValues::new(kind),
            present: BitVec::new(),
        }
    }

    /// Empties the column and makes it hold values of `kind`.
    fn restart(&mut self, kind: PropKind) {
        self.kind = kind;
        self.values.restart(kind);
        self.present.clear();
    }

    /// Makes the column hold exactly `len` features, the ones it had or none.
    fn resize(&mut self, len: usize) {
        match &mut self.values {
            ColumnValues::Scalars(bits) => bits.resize(len, 0),
            ColumnValues::Strs(strs) => strs.resize(len, ""),
        }
        self.present.resize(len, false);
    }

    /// Sets the value of `feature`, replacing the one it already has.
    fn set(&mut self, feature: usize, value: PropValueRef<'a>) {
        if feature >= self.present.len() {
            self.resize(feature + 1);
        }
        match (&mut self.values, scalar_bits(value)) {
            (ColumnValues::Scalars(bits), Ok(value)) => bits[feature] = value,
            (ColumnValues::Strs(strs), Err(text)) => strs[feature] = text,
            _ => unreachable!("the writer checked the value is of the column's kind"),
        }
        self.present.set(feature, true);
    }

    /// Drops every feature from `len` on.
    fn truncate(&mut self, len: usize) {
        self.values.truncate(len);
        self.present.truncate(len);
    }

    fn get(&self, feature: usize) -> Option<PropValueRef<'_>> {
        if !self.present[feature] {
            return None;
        }
        match &self.values {
            ColumnValues::Scalars(bits) => value_of_bits(self.kind, bits[feature]),
            ColumnValues::Strs(strs) => Some(PropValueRef::Str(strs[feature])),
        }
    }
}

#[cfg(feature = "unstable-v2")]
impl<'a> ColumnValues<'a> {
    /// Appends `value`, which is of the kind the values hold.
    fn push(&mut self, value: PropValueRef<'a>) {
        match (self, scalar_bits(value)) {
            (Self::Scalars(bits), Ok(value)) => bits.push(value),
            (Self::Strs(strs), Err(text)) => strs.push(text),
            _ => unreachable!("the writer checked the value is of the column's kind"),
        }
    }
}

/// The m-value columns of a layer, and the columns of an earlier layer kept for their buffers.
#[cfg(feature = "unstable-v2")]
#[derive(Debug, Clone, Default)]
struct MValueColumns<'a> {
    names: Vec<&'a str>,
    kinds: Vec<PropKind>,
    columns: Vec<MValueColumn<'a>>,
    spare: Vec<MValueColumn<'a>>,
}

/// One run of values, one per stored vertex, or none per feature, flat across features.
#[cfg(feature = "unstable-v2")]
#[derive(Debug, Clone)]
struct MValueColumn<'a> {
    values: ColumnValues<'a>,
    /// The cumulative end of each finished feature's run in `values`.
    ends: Vec<u32>,
    /// Whether each feature has a run, the one being written included once it sets one.
    present: BitVec<u8>,
}

#[cfg(feature = "unstable-v2")]
impl<'a> MValueColumns<'a> {
    /// Starts over with no columns, keeping their buffers.
    fn clear(&mut self) {
        self.names.clear();
        self.kinds.clear();
        self.spare.append(&mut self.columns);
    }

    /// Adds a column of `kind` that the `features` written so far are null in.
    fn add(&mut self, name: &'a str, kind: PropKind, features: usize) -> MValueKey {
        let mut column = self.spare.pop().unwrap_or_else(|| MValueColumn {
            values: ColumnValues::new(kind),
            ends: Vec::new(),
            present: BitVec::new(),
        });
        column.values.restart(kind);
        column.ends.clear();
        column.ends.resize(features, 0);
        column.present.clear();
        column.present.resize(features, false);
        self.columns.push(column);
        self.names.push(name);
        self.kinds.push(kind);
        MValueKey::new(self.names.len() - 1)
    }

    /// Checks that every run `feature` set holds one value per vertex it stores, which
    /// `vertices` counts.
    fn check_runs(&self, feature: usize, vertices: impl FnOnce() -> usize) -> MltResult<()> {
        let mut set = (self.columns.iter().enumerate())
            .filter(|(_, column)| column.present.len() > feature)
            .peekable();
        if set.peek().is_none() {
            return Ok(());
        }
        let expected = vertices();
        for (index, column) in set {
            let actual = column.values.len() - column.run_start();
            if actual != expected {
                return Err(MltError::MValueVertexCountMismatch {
                    index,
                    expected,
                    actual,
                });
            }
        }
        Ok(())
    }

    /// Ends the run of `feature` in every column, null where it set none.
    fn finish_feature(&mut self, feature: usize) -> MltResult<()> {
        for column in &mut self.columns {
            column.present.resize(feature + 1, false);
            column.ends.push(len32(column.values.len())?);
        }
        Ok(())
    }

    /// Drops every feature from `len` on, the one being written included.
    fn truncate(&mut self, len: usize) {
        for column in &mut self.columns {
            column.ends.truncate(len);
            column.values.truncate(column.run_start());
            column.present.truncate(len);
        }
    }
}

#[cfg(feature = "unstable-v2")]
impl MValueColumn<'_> {
    /// Where the run of the feature after the last finished one starts.
    fn run_start(&self) -> usize {
        self.ends.last().map_or(0, |&end| end as usize)
    }
}

/// Every feature's geometry as groups of parts of coordinates, each level holding the cumulative
/// end of its children.
#[derive(Debug, Clone, Default)]
struct Geometries {
    types: Vec<GeometryType>,
    feature_ends: Vec<u32>,
    group_ends: Vec<u32>,
    part_ends: Vec<u32>,
    coords: Vec<Coord<i32>>,
}

impl<'a> LayerWriter<'a> {
    pub fn new(name: &'a str, extent: u32) -> MltResult<Self> {
        validate_layer_name(name)?;
        Ok(Self {
            name,
            extent: Extent::new(extent)?,
            property_names: Vec::new(),
            property_kinds: Vec::new(),
            columns: Vec::new(),
            spare_columns: Vec::new(),
            ids: Vec::new(),
            geometry: Geometries::default(),
            #[cfg(feature = "unstable-v2")]
            m_values: MValueColumns::default(),
        })
    }

    /// Starts over as an empty layer, keeping the buffers of this one.
    pub fn reset(&mut self, name: &'a str, extent: u32) -> MltResult<()> {
        validate_layer_name(name)?;
        self.extent = Extent::new(extent)?;
        self.name = name;
        self.property_kinds.clear();
        self.property_names.clear();
        self.spare_columns.append(&mut self.columns);
        self.ids.clear();
        self.geometry.clear();
        #[cfg(feature = "unstable-v2")]
        self.m_values.clear();
        Ok(())
    }

    /// Every column name in use, with the kind of column that holds it.
    fn taken_names(&self) -> impl Iterator<Item = (&'a str, ColumnRole)> + '_ {
        let taken = self
            .property_names
            .iter()
            .map(|&n| (n, ColumnRole::Property));
        #[cfg(feature = "unstable-v2")]
        let taken = taken.chain(self.m_values.names.iter().map(|&n| (n, ColumnRole::MValue)));
        taken
    }

    /// Adds a property column, which the features written so far have no value in.
    pub fn add_property(&mut self, name: &'a str, kind: PropKind) -> MltResult<PropertyKey> {
        reject_taken_name(name, ColumnRole::Property, self.taken_names())?;
        let mut column = self
            .spare_columns
            .pop()
            .unwrap_or_else(|| Column::new(kind));
        column.restart(kind);
        column.resize(self.feature_count());
        self.columns.push(column);
        self.property_names.push(name);
        self.property_kinds.push(kind);
        Ok(PropertyKey::new(self.property_names.len() - 1))
    }

    /// Adds an m-value column, holding one value per stored vertex, which the features written
    /// so far are null in.
    #[cfg(feature = "unstable-v2")]
    pub fn add_m_value(&mut self, name: &'a str, kind: PropKind) -> MltResult<MValueKey> {
        reject_taken_name(name, ColumnRole::MValue, self.taken_names())?;
        let features = self.feature_count();
        Ok(self.m_values.add(name, kind, features))
    }

    /// Starts a feature of `geometry_type`. It joins the layer when [`FeatureWriter::finish`]
    /// succeeds, and is discarded otherwise.
    pub fn feature<'w>(&'w mut self, geometry_type: GeometryType) -> FeatureWriter<'w, 'a> {
        FeatureWriter {
            start: self.geometry.start(),
            layer: self,
            geometry_type,
            id: None,
            finished: false,
        }
    }

    #[must_use]
    pub fn feature_count(&self) -> usize {
        self.ids.len()
    }

    /// Encodes the layer as MLT, choosing every encoding as [`TileLayer::encode`] does.
    ///
    /// [`TileLayer::encode`]: crate::TileLayer::encode
    pub fn encode(&self, cfg: EncoderConfig) -> MltResult<Vec<u8>> {
        encode_layer(self, cfg)
    }

    /// Adds the layer to an MVT tile, as [`tile_layers_to_mvt`] would.
    ///
    /// [`tile_layers_to_mvt`]: crate::mvt::tile_layers_to_mvt
    pub fn write_mvt(&self, tile: MvtTileBuilder) -> MltResult<MvtTileBuilder> {
        write_mvt_layer(self, tile)
    }
}

/// A feature being written into a [`LayerWriter`].
#[must_use = "call .finish() to add the feature to the layer"]
pub struct FeatureWriter<'w, 'a> {
    layer: &'w mut LayerWriter<'a>,
    geometry_type: GeometryType,
    id: Option<u64>,
    /// Where this feature's geometry starts, to check its shape and to discard it.
    start: FeatureStart,
    finished: bool,
}

impl<'a> FeatureWriter<'_, 'a> {
    pub fn id(&mut self, id: Option<u64>) -> &mut Self {
        self.id = id;
        self
    }

    /// Sets the feature's value in the `key` column, which must be of the value's kind.
    pub fn property(
        &mut self,
        key: PropertyKey,
        value: impl Into<PropValueRef<'a>>,
    ) -> MltResult<&mut Self> {
        let value = value.into();
        let index = key.index();
        let layer = &mut *self.layer;
        let Some(&expected) = layer.property_kinds.get(index) else {
            return Err(MltError::UnknownProperty {
                index,
                count: layer.property_kinds.len(),
            });
        };
        let actual = value.kind();
        if actual != expected {
            return Err(MltError::PropertyKindMismatch {
                index,
                expected,
                actual,
            });
        }
        layer.columns[index].set(layer.ids.len(), value);
        Ok(self)
    }

    /// Sets the feature's values in the `key` m-value column, one per vertex it stores (a ring's
    /// closing coordinate is not stored), each of the column's kind.
    ///
    /// A feature that sets none is null in the column, as is one whose values are rejected.
    #[cfg(feature = "unstable-v2")]
    pub fn m_values(
        &mut self,
        key: MValueKey,
        values: impl IntoIterator<Item = impl Into<PropValueRef<'a>>>,
    ) -> MltResult<&mut Self> {
        let index = key.index();
        let feature = self.layer.ids.len();
        let m_values = &mut self.layer.m_values;
        let Some(&expected) = m_values.kinds.get(index) else {
            return Err(MltError::UnknownMValueKey {
                index,
                columns: m_values.kinds.len(),
            });
        };
        let column = &mut m_values.columns[index];
        column.values.truncate(column.run_start());
        column.present.resize(feature, false);
        for value in values {
            let value = value.into();
            let actual = value.kind();
            if actual != expected {
                column.values.truncate(column.run_start());
                return Err(MltError::MValueKindMismatch {
                    index,
                    expected,
                    actual,
                });
            }
            column.values.push(value);
        }
        column.present.push(true);
        Ok(self)
    }

    /// Adds points to a [`GeometryType::Point`] (which holds exactly one) or
    /// [`GeometryType::MultiPoint`] feature.
    pub fn points(&mut self, coords: impl IntoIterator<Item = Coord<i32>>) -> MltResult<&mut Self> {
        self.check_family(Family::Points, "points")?;
        let geometry = &mut self.layer.geometry;
        if geometry.group_ends.len() == self.start.groups {
            geometry.begin_group()?;
            geometry.begin_part()?;
        }
        geometry.extend(coords)?;
        Ok(self)
    }

    /// Adds a line to a [`GeometryType::LineString`] (which holds exactly one) or
    /// [`GeometryType::MultiLineString`] feature.
    pub fn line(&mut self, coords: impl IntoIterator<Item = Coord<i32>>) -> MltResult<&mut Self> {
        self.check_family(Family::Lines, "lines")?;
        let geometry = &mut self.layer.geometry;
        if geometry.group_ends.len() == self.start.groups {
            geometry.begin_group()?;
        }
        geometry.begin_part()?;
        geometry.extend(coords)?;
        Ok(self)
    }

    /// Starts a polygon of a [`GeometryType::Polygon`] (which holds exactly one) or
    /// [`GeometryType::MultiPolygon`] feature with its exterior ring, which may be open or closed.
    pub fn exterior_ring(
        &mut self,
        coords: impl IntoIterator<Item = Coord<i32>>,
    ) -> MltResult<&mut Self> {
        self.check_family(Family::Polygons, "rings")?;
        self.layer.geometry.begin_group()?;
        self.ring(coords)
    }

    /// Adds a hole to the polygon the last [`Self::exterior_ring`] started, which may be open or
    /// closed.
    pub fn hole(&mut self, coords: impl IntoIterator<Item = Coord<i32>>) -> MltResult<&mut Self> {
        self.check_family(Family::Polygons, "rings")?;
        if self.layer.geometry.group_ends.len() == self.start.groups {
            return Err(MltError::InvalidFeatureGeometry(
                self.geometry_type,
                "starts with a hole instead of an exterior ring",
            ));
        }
        self.ring(coords)
    }

    fn ring(&mut self, coords: impl IntoIterator<Item = Coord<i32>>) -> MltResult<&mut Self> {
        let geometry = &mut self.layer.geometry;
        geometry.begin_part()?;
        geometry.extend(coords)?;
        Ok(self)
    }

    fn check_family(&self, family: Family, what: &'static str) -> MltResult<()> {
        if Family::of(self.geometry_type).0 == family {
            Ok(())
        } else {
            Err(MltError::InvalidFeatureGeometry(self.geometry_type, what))
        }
    }

    /// Adds the feature to the layer, or discards it if its geometry is not of its type.
    pub fn finish(mut self) -> MltResult<()> {
        let layer = &mut *self.layer;
        let geometry = &layer.geometry;
        let invalid = match Family::of(self.geometry_type) {
            _ if self.geometry_type.is_mesh() => {
                Some("has triangles, which a layer writer cannot take")
            }
            (_, true) => None,
            (Family::Points, false) => (geometry.coords.len() - self.start.coords != 1)
                .then_some("needs exactly one point"),
            (Family::Lines, false) => (geometry.part_ends.len() - self.start.parts != 1)
                .then_some("needs exactly one line"),
            (Family::Polygons, false) => (geometry.group_ends.len() - self.start.groups != 1)
                .then_some("needs exactly one polygon"),
        };
        if let Some(reason) = invalid {
            return Err(MltError::InvalidFeatureGeometry(self.geometry_type, reason));
        }
        #[cfg(feature = "unstable-v2")]
        {
            let feature = layer.ids.len();
            let rings = Family::of(self.geometry_type).0 == Family::Polygons;
            layer
                .m_values
                .check_runs(feature, || geometry.stored_since(self.start, rings))?;
            layer.m_values.finish_feature(feature)?;
        }
        // The last step that can fail, since a discarded feature leaves its geometry types behind.
        layer.geometry.finish_feature(self.geometry_type)?;
        let len = layer.ids.len() + 1;
        for column in &mut layer.columns {
            column.resize(len);
        }
        layer.ids.push(self.id);
        self.finished = true;
        Ok(())
    }
}

impl Drop for FeatureWriter<'_, '_> {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        let layer = &mut *self.layer;
        let feature = layer.ids.len();
        for column in &mut layer.columns {
            column.truncate(feature);
        }
        #[cfg(feature = "unstable-v2")]
        layer.m_values.truncate(feature);
        layer.geometry.rewind(self.start);
    }
}

/// The kind of parts a geometry type is made of, and whether it repeats them.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Family {
    Points,
    Lines,
    Polygons,
}

impl Family {
    /// A mesh's vertices are a `MultiPoint`'s, though [`FeatureWriter::finish`] rejects it.
    fn of(geometry_type: GeometryType) -> (Self, bool) {
        match geometry_type {
            GeometryType::Point => (Self::Points, false),
            GeometryType::MultiPoint => (Self::Points, true),
            #[cfg(feature = "unstable-v2")]
            GeometryType::Mesh => (Self::Points, true),
            GeometryType::LineString => (Self::Lines, false),
            GeometryType::MultiLineString => (Self::Lines, true),
            GeometryType::Polygon => (Self::Polygons, false),
            GeometryType::MultiPolygon => (Self::Polygons, true),
        }
    }
}

/// How much of each [`Geometries`] level was in use when a feature started.
#[derive(Debug, Clone, Copy)]
struct FeatureStart {
    groups: usize,
    parts: usize,
    coords: usize,
}

impl Geometries {
    fn clear(&mut self) {
        self.rewind(FeatureStart {
            groups: 0,
            parts: 0,
            coords: 0,
        });
        self.types.clear();
        self.feature_ends.clear();
    }

    fn start(&self) -> FeatureStart {
        FeatureStart {
            groups: self.group_ends.len(),
            parts: self.part_ends.len(),
            coords: self.coords.len(),
        }
    }

    fn rewind(&mut self, start: FeatureStart) {
        self.group_ends.truncate(start.groups);
        self.part_ends.truncate(start.parts);
        self.coords.truncate(start.coords);
    }

    fn begin_group(&mut self) -> MltResult<()> {
        self.group_ends.push(len32(self.part_ends.len())?);
        Ok(())
    }

    fn begin_part(&mut self) -> MltResult<()> {
        self.part_ends.push(len32(self.coords.len())?);
        *self
            .group_ends
            .last_mut()
            .expect("a part belongs to a group") += 1;
        Ok(())
    }

    fn extend(&mut self, coords: impl IntoIterator<Item = Coord<i32>>) -> MltResult<()> {
        self.coords.extend(coords);
        *self
            .part_ends
            .last_mut()
            .expect("coordinates belong to a part") = len32(self.coords.len())?;
        Ok(())
    }

    fn finish_feature(&mut self, geometry_type: GeometryType) -> MltResult<()> {
        self.feature_ends.push(len32(self.group_ends.len())?);
        self.types.push(geometry_type);
        Ok(())
    }

    /// How many vertices the feature started at `start` stores, which drops a closing coordinate
    /// of each of its parts when they are `rings`.
    #[cfg(feature = "unstable-v2")]
    fn stored_since(&self, start: FeatureStart, rings: bool) -> usize {
        if !rings {
            return self.coords.len() - start.coords;
        }
        (start.parts..self.part_ends.len())
            .map(|part| stored_len(self.part(part)))
            .sum()
    }

    fn groups(&self, feature: usize) -> Range<usize> {
        item_range(&self.feature_ends, feature)
    }

    /// The parts of consecutive `groups`.
    fn parts(&self, groups: Range<usize>) -> Range<usize> {
        if groups.is_empty() {
            return 0..0;
        }
        item_range(&self.group_ends, groups.start).start..self.group_ends[groups.end - 1] as usize
    }

    fn part(&self, part: usize) -> &[Coord<i32>] {
        &self.coords[item_range(&self.part_ends, part)]
    }

    fn feature_coords(&self, feature: usize) -> &[Coord<i32>] {
        let parts = self.parts(self.groups(feature));
        if parts.is_empty() {
            return &[];
        }
        &self.coords
            [item_range(&self.part_ends, parts.start).start..self.part_ends[parts.end - 1] as usize]
    }
}

impl LayerSource for LayerWriter<'_> {
    fn name(&self) -> &str {
        self.name
    }

    fn extent(&self) -> Extent {
        self.extent
    }

    fn feature_count(&self) -> usize {
        Self::feature_count(self)
    }

    fn property_count(&self) -> usize {
        self.property_names.len()
    }

    fn property_name(&self, column: usize) -> &str {
        self.property_names[column]
    }

    fn property_kinds(&self) -> &[PropKind] {
        &self.property_kinds
    }

    fn id(&self, feature: usize) -> Option<u64> {
        self.ids[feature]
    }

    fn property(&self, feature: usize, column: usize) -> Option<PropValueRef<'_>> {
        self.columns[column].get(feature)
    }

    fn coords(&self) -> impl Iterator<Item = Coord<i32>> + '_ {
        self.geometry.coords.iter().copied()
    }

    fn first_coord(&self, feature: usize) -> Option<Coord<i32>> {
        self.geometry.feature_coords(feature).first().copied()
    }

    fn coord_count(&self, feature: usize) -> usize {
        self.geometry.feature_coords(feature).len()
    }

    fn push_geometry(&self, feature: usize, values: &mut GeometryValues) {
        let geometry = &self.geometry;
        let groups = geometry.groups(feature);
        let parts = geometry.parts(groups.clone());
        let part = |p| geometry.part(p);
        match geometry.types[feature] {
            GeometryType::Point => values.push_point(geometry.feature_coords(feature)[0]),
            GeometryType::MultiPoint => {
                values.push_multi_point(geometry.feature_coords(feature).iter().copied());
            }
            GeometryType::LineString => values.push_linestring(part(parts.start)),
            GeometryType::MultiLineString => values.push_multi_linestring(parts.map(part)),
            GeometryType::Polygon => values.push_polygon(parts.map(part)),
            GeometryType::MultiPolygon => values
                .push_multi_polygon(groups.map(|group| geometry.parts(group..group + 1).map(part))),
            #[cfg(feature = "unstable-v2")]
            GeometryType::Mesh => unreachable!("FeatureWriter::finish rejects meshes"),
        }
    }

    fn polygons(
        &self,
        feature: usize,
    ) -> impl Iterator<Item = impl Iterator<Item = &[Coord<i32>]> + '_> + '_ {
        let geometry = &self.geometry;
        // Each polygon is one group, whose parts are its rings.
        let groups = match Family::of(geometry.types[feature]).0 {
            Family::Polygons => geometry.groups(feature),
            Family::Points | Family::Lines => 0..0,
        };
        groups.map(move |group| {
            geometry
                .parts(group..group + 1)
                .map(move |part| geometry.part(part))
        })
    }

    #[cfg(feature = "unstable-v2")]
    fn m_value_kinds(&self) -> &[PropKind] {
        &self.m_values.kinds
    }

    #[cfg(feature = "unstable-v2")]
    fn m_value_name(&self, column: usize) -> &str {
        self.m_values.names[column]
    }

    #[cfg(feature = "unstable-v2")]
    fn push_m_values(&self, feature: usize, column: usize, values: &mut StagedValues) -> bool {
        let column = &self.m_values.columns[column];
        if !column.present[feature] {
            return false;
        }
        let run = item_range(&column.ends, feature);
        let moved = self.vertex_order(feature);
        match (&column.values, values) {
            (ColumnValues::Strs(strs), StagedValues::Str(out)) => {
                extend_run(out, &strs[run], moved, |&text| CompactString::from(text));
            }
            (ColumnValues::Scalars(bits), out) => extend_bits_run(out, &bits[run], moved),
            (ColumnValues::Strs(_), _) => unreachable!("m-value kind matches its column"),
        }
        true
    }

    #[cfg(feature = "unstable-v2")]
    fn vertex_order(&self, feature: usize) -> Option<Vec<usize>> {
        wound_polygons_order(LayerSource::polygons(self, feature))
    }

    fn mvt_feature(&self, feature: usize, layer: MvtLayerBuilder) -> MltResult<MvtFeatureBuilder> {
        let geometry = &self.geometry;
        let groups = geometry.groups(feature);
        let part = |p| geometry.part(p).iter().copied();
        Ok(match Family::of(geometry.types[feature]).0 {
            Family::Points => {
                let mut out = layer.feature_of(MvtGeomType::Point);
                out.points(geometry.feature_coords(feature).iter().copied())?;
                out
            }
            Family::Lines => {
                let mut out = layer.feature_of(MvtGeomType::LineString);
                for p in geometry.parts(groups) {
                    out.line(part(p))?;
                }
                out
            }
            Family::Polygons => {
                let mut out = layer.feature_of(MvtGeomType::Polygon);
                for group in groups {
                    let rings = geometry.parts(group..group + 1);
                    for p in rings.clone() {
                        out.ring(part(p), p == rings.start)?;
                    }
                }
                out
            }
        })
    }
}

/// The bits of a scalar `value`, or its text.
fn scalar_bits(value: PropValueRef<'_>) -> Result<u64, &str> {
    Ok(match value {
        PropValueRef::Bool(v) => u64::from(v),
        PropValueRef::I8(v) => i64::from(v).cast_unsigned(),
        PropValueRef::U8(v) => u64::from(v),
        PropValueRef::I32(v) => i64::from(v).cast_unsigned(),
        PropValueRef::U32(v) => u64::from(v),
        PropValueRef::I64(v) => v.cast_unsigned(),
        PropValueRef::U64(v) => v,
        PropValueRef::F32(v) => u64::from(v.to_bits()),
        PropValueRef::F64(v) => v.to_bits(),
        PropValueRef::Str(text) => return Err(text),
    })
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the bits were stored from a value of `kind`"
)]
/// The value of a scalar `kind` stored as `bits`, or [`None`] for a string kind.
fn value_of_bits(kind: PropKind, bits: u64) -> Option<PropValueRef<'static>> {
    Some(match kind {
        PropKind::Bool => PropValueRef::Bool(bits != 0),
        PropKind::I8 => PropValueRef::I8(bits.cast_signed() as i8),
        PropKind::U8 => PropValueRef::U8(bits as u8),
        PropKind::I32 => PropValueRef::I32(bits.cast_signed() as i32),
        PropKind::U32 => PropValueRef::U32(bits as u32),
        PropKind::I64 => PropValueRef::I64(bits.cast_signed()),
        PropKind::U64 => PropValueRef::U64(bits),
        PropKind::F32 => PropValueRef::F32(f32::from_bits(bits as u32)),
        PropKind::F64 => PropValueRef::F64(f64::from_bits(bits)),
        PropKind::Str => return None,
    })
}

/// Appends a feature's run of scalar values stored as `bits` to `out`, which holds their kind,
/// read through `moved` as [`extend_run`] does.
#[cfg(feature = "unstable-v2")]
#[expect(
    clippy::cast_possible_truncation,
    reason = "the bits were stored from a value of the column's kind"
)]
fn extend_bits_run(out: &mut StagedValues, bits: &[u64], moved: Option<Vec<usize>>) {
    match out {
        StagedValues::Bool(out) => extend_run(out, bits, moved, |&b| b != 0),
        StagedValues::I8(out) => extend_run(out, bits, moved, |&b| b.cast_signed() as i8),
        StagedValues::U8(out) => extend_run(out, bits, moved, |&b| b as u8),
        StagedValues::I32(out) => extend_run(out, bits, moved, |&b| b.cast_signed() as i32),
        StagedValues::U32(out) => extend_run(out, bits, moved, |&b| b as u32),
        StagedValues::I64(out) => extend_run(out, bits, moved, |&b| b.cast_signed()),
        StagedValues::U64(out) => extend_run(out, bits, moved, |&b| b),
        StagedValues::F32(out) => extend_run(out, bits, moved, |&b| f32::from_bits(b as u32)),
        StagedValues::F64(out) => extend_run(out, bits, moved, |&b| f64::from_bits(b)),
        StagedValues::Str(_) => unreachable!("m-value kind matches its column"),
    }
}

/// The range of item `index` among cumulative `ends`.
fn item_range(ends: &[u32], index: usize) -> Range<usize> {
    let start = index.checked_sub(1).map_or(0, |prev| ends[prev] as usize);
    start..ends[index] as usize
}

/// A layer holds at most `u32::MAX` vertices.
fn len32(len: usize) -> MltResult<u32> {
    u32::try_from(len).map_err(|_| MltError::IntegerOverflow)
}

#[cfg(test)]
mod tests {
    use geo_types::{Geometry, Point};

    use super::*;
    use crate::encoder::{Network, Profile};
    use crate::{PropValue, TileLayer};

    fn c(x: i32, y: i32) -> Coord<i32> {
        Coord { x, y }
    }

    fn encode(writer: &LayerWriter<'_>) -> Vec<u8> {
        writer
            .encode(EncoderConfig::from_profile(Profile::Native(
                Network::Cellular4G,
            )))
            .expect("encode")
    }

    fn two_points(writer: &mut LayerWriter<'static>) -> MltResult<()> {
        writer.reset("points", 4096)?;
        let name = writer.add_property("name", PropKind::Str)?;
        let rank = writer.add_property("rank", PropKind::U32)?;
        let mut feature = writer.feature(GeometryType::Point);
        feature.id(Some(1)).points([c(1, 2)])?;
        feature.property(name, "a")?.property(rank, 7_u32)?;
        feature.finish()?;
        let mut feature = writer.feature(GeometryType::Point);
        feature.points([c(3, 4)])?.property(name, "b")?;
        feature.finish()
    }

    #[test]
    fn rejects_extra_points_in_a_point() {
        let mut writer = LayerWriter::new("l", 4096).unwrap();
        let mut feature = writer.feature(GeometryType::Point);
        feature.points([c(1, 1), c(2, 2)]).unwrap();
        insta::assert_snapshot!(feature.finish().unwrap_err(), @"a Point feature needs exactly one point");
        assert_eq!(writer.feature_count(), 0);
    }

    #[test]
    fn rejects_empty_line_string() {
        let mut writer = LayerWriter::new("l", 4096).unwrap();
        insta::assert_snapshot!(
            writer.feature(GeometryType::LineString).finish().unwrap_err(),
            @"a LineString feature needs exactly one line"
        );
        assert_eq!(writer.feature_count(), 0);
    }

    #[test]
    fn rejects_hole_before_exterior_ring() {
        let mut writer = LayerWriter::new("l", 4096).unwrap();
        let mut feature = writer.feature(GeometryType::Polygon);
        insta::assert_snapshot!(feature.hole([c(0, 0)]).err().unwrap(), @"a Polygon feature starts with a hole instead of an exterior ring");
    }

    #[test]
    fn rejects_line_in_polygon() {
        let mut writer = LayerWriter::new("l", 4096).unwrap();
        let mut feature = writer.feature(GeometryType::Polygon);
        insta::assert_snapshot!(feature.line([c(0, 0)]).err().unwrap(), @"a Polygon feature lines");
    }

    #[test]
    fn rejects_empty_polygon() {
        let mut writer = LayerWriter::new("l", 4096).unwrap();
        insta::assert_snapshot!(
            writer.feature(GeometryType::Polygon).finish().unwrap_err(),
            @"a Polygon feature needs exactly one polygon"
        );
        assert_eq!(writer.feature_count(), 0);
    }

    #[test]
    fn accepts_empty_multi_polygon() {
        let mut writer = LayerWriter::new("l", 4096).unwrap();
        writer.feature(GeometryType::MultiPolygon).finish().unwrap();
        assert_eq!(writer.feature_count(), 1);
    }

    #[test]
    fn rejects_value_of_another_kind() {
        let mut writer = LayerWriter::new("l", 4096).unwrap();
        let rank = writer.add_property("rank", PropKind::U32).unwrap();
        let mut feature = writer.feature(GeometryType::Point);
        insta::assert_snapshot!(feature.property(rank, 7_i32).err().unwrap(), @"property 0 kind mismatch: expected U32, got I32");
    }

    #[test]
    fn rejects_unknown_property_key() {
        let mut writer = LayerWriter::new("l", 4096).unwrap();
        writer.add_property("rank", PropKind::U32).unwrap();
        let mut feature = writer.feature(GeometryType::Point);
        insta::assert_snapshot!(
            feature.property(PropertyKey::new(1), 7_u32).err().unwrap(),
            @"property 1 does not exist: the layer has 1"
        );
    }

    #[test]
    fn discards_unfinished_features() {
        let mut expected = LayerWriter::new("l", 4096).unwrap();
        two_points(&mut expected).unwrap();
        let mut writer = LayerWriter::new("l", 4096).unwrap();
        two_points(&mut writer).unwrap();
        let mut feature = writer.feature(GeometryType::MultiPoint);
        feature
            .points([c(9, 9)])
            .unwrap()
            .property(PropertyKey::new(0), "gone")
            .unwrap();
        drop(feature);
        assert_eq!(encode(&writer), encode(&expected));
    }

    #[test]
    fn reset_writer_encodes_like_a_new_one() {
        let mut fresh = LayerWriter::new("x", 4096).unwrap();
        two_points(&mut fresh).unwrap();
        let mut reused = LayerWriter::new("other", 512).unwrap();
        reused.add_property("unrelated", PropKind::F64).unwrap();
        reused.add_property("text", PropKind::Str).unwrap();
        let mut feature = reused.feature(GeometryType::LineString);
        feature.line([c(0, 0), c(5, 5)]).unwrap();
        feature
            .property(PropertyKey::new(1), "longer text")
            .unwrap();
        feature.finish().unwrap();
        two_points(&mut reused).unwrap();
        assert_eq!(encode(&reused), encode(&fresh));
    }

    #[test]
    fn column_added_after_features_encodes_like_tile_layer() {
        let mut writer = LayerWriter::new("l", 4096).unwrap();
        let mut feature = writer.feature(GeometryType::Point);
        feature.points([c(1, 1)]).unwrap();
        feature.finish().unwrap();
        let flag = writer.add_property("flag", PropKind::Bool).unwrap();
        let mut feature = writer.feature(GeometryType::Point);
        feature
            .points([c(2, 2)])
            .unwrap()
            .property(flag, true)
            .unwrap();
        feature.finish().unwrap();

        let mut layer = TileLayer::builder("l", 4096).unwrap();
        layer
            .feature(Geometry::Point(Point::new(1, 1)))
            .finish()
            .unwrap();
        let flag = layer.add_property("flag", PropKind::Bool).unwrap();
        let mut feature = layer.feature(Geometry::Point(Point::new(2, 2)));
        feature.property(flag, PropValue::Bool(Some(true))).unwrap();
        feature.finish().unwrap();
        let layer = layer.finish();
        assert_eq!(
            encode(&writer),
            layer
                .encode(EncoderConfig::from_profile(Profile::Native(
                    Network::Cellular4G
                )))
                .unwrap()
        );
    }

    #[test]
    fn repeated_property_keeps_the_last_value() {
        let mut writer = LayerWriter::new("l", 4096).unwrap();
        let flag = writer.add_property("flag", PropKind::Bool).unwrap();
        let mut feature = writer.feature(GeometryType::Point);
        feature
            .points([c(2, 2)])
            .unwrap()
            .property(flag, false)
            .unwrap()
            .property(flag, true)
            .unwrap();
        feature.finish().unwrap();

        let mut layer = TileLayer::builder("l", 4096).unwrap();
        let flag = layer.add_property("flag", PropKind::Bool).unwrap();
        let mut feature = layer.feature(Geometry::Point(Point::new(2, 2)));
        feature.property(flag, PropValue::Bool(Some(true))).unwrap();
        feature.finish().unwrap();
        let layer = layer.finish();
        assert_eq!(
            encode(&writer),
            layer
                .encode(EncoderConfig::from_profile(Profile::Native(
                    Network::Cellular4G
                )))
                .unwrap()
        );
    }
}
