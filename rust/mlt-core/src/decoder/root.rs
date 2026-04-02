use crate::LazyParsed::Raw;
use crate::MltError::{
    BufferUnderflow, GeometryWithoutStreams, InvalidSharedDictStreamCount, MissingGeometry,
    MultipleGeometryColumns, MultipleIdColumns, SharedDictRequiresStreams, TrailingLayerData,
    UnexpectedStructChildCount, UnsupportedStringStreamCount,
};
use crate::codecs::varint::parse_varint;
use crate::decoder::{
    Column, ColumnType, DictionaryType, Geometry, GeometryValues, Id, IdValues, Layer01,
    ParsedLayer01, RawFsstData, RawGeometry, RawId, RawIdValue, RawPlainData, RawPresence,
    RawProperty, RawScalar, RawSharedDict, RawSharedDictEncoding, RawSharedDictItem, RawStream,
    RawStrings, RawStringsEncoding, StreamType,
};
use crate::errors::AsMltError as _;
use crate::utils::{AsUsize as _, SetOptionOnce as _, parse_string};
use crate::{Layer, Lazy, MltError, MltRefResult, MltResult, ParsedLayer};

/// Default memory budget: 20 MiB.
const DEFAULT_MAX_BYTES: u32 = 20 * 1024 * 1024;

/// Stateful decoder that enforces a per-tile memory budget during decoding.
///
/// Pass a `Decoder` to every `raw.decode()` / `into_tile()` call and to
/// `from_bytes`-style parsers. Each method charges the budget before
/// performing heap allocations, so the total heap used never exceeds `max_bytes`
/// (in bytes).
///
/// ```
/// use mlt_core::Decoder;
///
/// // Default: 10 MiB budget.
/// let mut dec = Decoder::default();
///
/// // Custom budget.
/// let mut dec = Decoder::with_max_size(64 * 1024 * 1024);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Decoder {
    /// Keep track of the memory used when decoding a tile: raw->parsed transition
    budget: MemBudget,
    /// Reusable scratch buffer for the physical u32 decode pass.
    /// Held here so its heap allocation is reused across streams without extra cost.
    pub(crate) buffer_u32: Vec<u32>,
    /// Reusable scratch buffer for the physical u64 decode pass.
    /// Held here so its heap allocation is reused across streams without extra cost.
    pub(crate) buffer_u64: Vec<u64>,
}

impl Decoder {
    /// Create a decoder with a custom memory budget (in bytes).
    #[must_use]
    pub fn with_max_size(max_bytes: u32) -> Self {
        Self {
            budget: MemBudget::with_max_size(max_bytes),
            ..Default::default()
        }
    }

    pub fn decode_all<'a>(
        &mut self,
        layers: impl IntoIterator<Item = Layer<'a>>,
    ) -> MltResult<Vec<ParsedLayer<'a>>> {
        layers
            .into_iter()
            .map(|l| l.decode_all(self))
            .collect::<MltResult<_>>()
    }

    /// Allocate a `Vec<T>` with the given capacity, charging the decoder's budget for
    /// `capacity * size_of::<T>()` bytes. Use this instead of `Vec::with_capacity` in decode paths.
    #[inline]
    pub(crate) fn alloc<T>(&mut self, capacity: usize) -> MltResult<Vec<T>> {
        let bytes = capacity.checked_mul(size_of::<T>()).or_overflow()?;
        let bytes_u32 = u32::try_from(bytes).or_overflow()?;
        self.budget.consume(bytes_u32)?;
        Ok(Vec::with_capacity(capacity))
    }

    /// Charge the budget for `size` raw bytes. Prefer [`consume_items`][Self::consume_items]
    /// when charging for a known-type collection.
    #[inline]
    pub(crate) fn consume(&mut self, size: u32) -> MltResult<()> {
        self.budget.consume(size)
    }

    /// Charge the budget for `count` items of type `T` (`count * size_of::<T>()` bytes).
    #[inline]
    pub(crate) fn consume_items<T>(&mut self, count: usize) -> MltResult<()> {
        let bytes = count.checked_mul(size_of::<T>()).or_overflow()?;
        self.budget.consume(u32::try_from(bytes).or_overflow()?)
    }

    #[inline]
    pub(crate) fn adjust(&mut self, adjustment: u32) {
        self.budget.adjust(adjustment);
    }

    /// Assert (in debug builds) that `buf` has not grown beyond `alloc_size`, then adjust the
    /// budget to return any bytes that were pre-charged but not actually used.
    ///
    /// Call this after fully populating a `Vec<T>` that was pre-allocated with [`Decoder::alloc`],
    /// passing the same `alloc_size` that was given to `alloc`.
    ///
    /// - Panics in debug builds if `buf.capacity() > alloc_size` (unexpected reallocation).
    /// - Subtracts `(alloc_size - buf.len()) * size_of::<T>()` from the budget (the pre-charged
    ///   bytes that correspond to capacity that was never filled).
    #[inline]
    pub(crate) fn adjust_alloc<T>(&mut self, buf: &Vec<T>, alloc_size: usize) {
        debug_assert!(
            buf.capacity() <= alloc_size,
            "Vector reallocated beyond initial allocation size ({alloc_size}); final capacity: {}",
            buf.capacity()
        );
        // Return the unused portion of the pre-charged budget.
        // alloc_size >= buf.len() is guaranteed by the assert above (capacity >= len always).
        let unused = (alloc_size - buf.len()) * size_of::<T>();
        // unused fits in u32: it's at most alloc_size * size_of::<T>(), which was checked to fit
        // in u32 when alloc() was called. Using saturating_cast to avoid a fallible conversion.
        #[expect(
            clippy::cast_possible_truncation,
            reason = "unused <= alloc_size * size_of::<T>() which was verified to fit in u32 by alloc()"
        )]
        self.budget.adjust(unused as u32);
    }

    #[must_use]
    pub fn consumed(&self) -> u32 {
        self.budget.consumed()
    }
}

/// Stateful parser that enforces a memory budget during parsing (binary → raw structures).
///
/// The parse chain reserves memory before allocations so total heap stays within the limit.
///
/// ```
/// use mlt_core::Parser;
///
/// # let bytes: &[u8] = &[];
/// let mut parser = Parser::default();
/// let layers = parser.parse_layers(bytes).expect("parse");
///
/// // Or with a custom limit:
/// let mut parser = Parser::with_max_size(64 * 1024 * 1024);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Parser {
    budget: MemBudget,
}

impl Parser {
    /// Create a parser with a custom memory budget (in bytes).
    #[must_use]
    pub fn with_max_size(max_bytes: u32) -> Self {
        Self {
            budget: MemBudget::with_max_size(max_bytes),
        }
    }

    /// Parse a sequence of binary layers, reserving decoded memory against this parser's budget.
    pub fn parse_layers<'a>(&mut self, mut input: &'a [u8]) -> MltResult<Vec<Layer<'a>>> {
        let mut result = Vec::new();
        while !input.is_empty() {
            let layer;
            (input, layer) = Layer::from_bytes(input, self)?;
            result.push(layer);
        }
        Ok(result)
    }

    /// Reserve `size` bytes from the parse budget. Used internally by the parse chain.
    #[inline]
    pub(crate) fn reserve(&mut self, size: u32) -> MltResult<()> {
        self.budget.consume(size)
    }

    #[must_use]
    pub fn reserved(&self) -> u32 {
        self.budget.consumed()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MemBudget {
    /// Hard ceiling: total decoded bytes may not exceed this value.
    pub max_bytes: u32,
    /// Running total of used bytes so far.
    pub bytes_used: u32,
}

impl Default for MemBudget {
    /// Create a decoder with the default 10 MiB memory budget.
    fn default() -> Self {
        Self::with_max_size(DEFAULT_MAX_BYTES)
    }
}

impl MemBudget {
    /// Create a decoder with a custom memory budget (in bytes).
    #[must_use]
    fn with_max_size(max_bytes: u32) -> Self {
        Self {
            max_bytes,
            bytes_used: 0,
        }
    }

    /// Adjust previous consumption by `- adjustment` bytes.  Will panic if used incorrectly.
    #[inline]
    fn adjust(&mut self, adjustment: u32) {
        self.bytes_used = self.bytes_used.checked_sub(adjustment).unwrap();
    }

    /// Take `size` bytes from the allocation budget. Call this before the actual allocation.
    #[inline]
    fn consume(&mut self, size: u32) -> MltResult<()> {
        let accumulator = &mut self.bytes_used;
        let max_bytes = self.max_bytes;
        if let Some(new_value) = accumulator
            .checked_add(size)
            .and_then(|v| if v > max_bytes { None } else { Some(v) })
        {
            *accumulator = new_value;
            Ok(())
        } else {
            Err(MltError::MemoryLimitExceeded {
                limit: max_bytes,
                used: *accumulator,
                requested: size,
            })
        }
    }

    fn consumed(&self) -> u32 {
        self.bytes_used
    }
}

impl<'a> Layer01<'a, Lazy> {
    /// Parse `v01::Layer` metadata, reserving decoded memory against the parser's budget.
    pub fn from_bytes(input: &'a [u8], parser: &mut Parser) -> MltResult<Self> {
        let (input, layer_name) = parse_string(input)?;
        let (input, extent) = parse_varint::<u32>(input)?;
        let (input, column_count) = parse_varint::<u32>(input)?;

        // Each column requires at least 1 byte (column type)
        if input.len() < column_count.as_usize() {
            return Err(BufferUnderflow(column_count, input.len()));
        }

        // !!!!!!!
        // WARNING: make sure to never use `let (input, ...)` after this point: input var is reused
        let (mut input, (col_info, prop_count)) = parse_columns_meta(input, column_count, parser)?;
        #[cfg(fuzzing)]
        let layer_order = col_info
            .iter()
            .map(|column| column.typ)
            .map(crate::decoder::fuzzing::LayerOrdering::from)
            .collect();

        let mut properties = Vec::with_capacity(prop_count.as_usize());
        let mut id_column: Option<Id> = None;
        let mut geometry: Option<Geometry> = None;

        for column in col_info {
            use crate::decoder::RawProperty as RP;

            let opt;
            let value;
            let name = column.name.unwrap_or("");

            match column.typ {
                ColumnType::Id | ColumnType::OptId => {
                    (input, opt) = parse_optional(column.typ, input, parser)?;
                    (input, value) = RawStream::from_bytes(input, parser)?;
                    id_column.set_once(Raw(RawId {
                        presence: RawPresence(opt),
                        value: RawIdValue::Id32(value),
                    }))?;
                }
                ColumnType::LongId | ColumnType::OptLongId => {
                    (input, opt) = parse_optional(column.typ, input, parser)?;
                    (input, value) = RawStream::from_bytes(input, parser)?;
                    id_column.set_once(Raw(RawId {
                        presence: RawPresence(opt),
                        value: RawIdValue::Id64(value),
                    }))?;
                }
                ColumnType::Geometry => {
                    input = parse_geometry_column(input, &mut geometry, parser)?;
                }
                ColumnType::Bool | ColumnType::OptBool => {
                    (input, opt) = parse_optional(column.typ, input, parser)?;
                    (input, value) = RawStream::parse_bool(input, parser)?;
                    properties.push(Raw(RP::Bool(scalar(name, opt, value))));
                }
                ColumnType::I8 | ColumnType::OptI8 => {
                    (input, opt) = parse_optional(column.typ, input, parser)?;
                    (input, value) = RawStream::from_bytes(input, parser)?;
                    properties.push(Raw(RP::I8(scalar(name, opt, value))));
                }
                ColumnType::U8 | ColumnType::OptU8 => {
                    (input, opt) = parse_optional(column.typ, input, parser)?;
                    (input, value) = RawStream::from_bytes(input, parser)?;
                    properties.push(Raw(RP::U8(scalar(name, opt, value))));
                }
                ColumnType::I32 | ColumnType::OptI32 => {
                    (input, opt) = parse_optional(column.typ, input, parser)?;
                    (input, value) = RawStream::from_bytes(input, parser)?;
                    properties.push(Raw(RP::I32(scalar(name, opt, value))));
                }
                ColumnType::U32 | ColumnType::OptU32 => {
                    (input, opt) = parse_optional(column.typ, input, parser)?;
                    (input, value) = RawStream::from_bytes(input, parser)?;
                    properties.push(Raw(RP::U32(scalar(name, opt, value))));
                }
                ColumnType::I64 | ColumnType::OptI64 => {
                    (input, opt) = parse_optional(column.typ, input, parser)?;
                    (input, value) = RawStream::from_bytes(input, parser)?;
                    properties.push(Raw(RP::I64(scalar(name, opt, value))));
                }
                ColumnType::U64 | ColumnType::OptU64 => {
                    (input, opt) = parse_optional(column.typ, input, parser)?;
                    (input, value) = RawStream::from_bytes(input, parser)?;
                    properties.push(Raw(RP::U64(scalar(name, opt, value))));
                }
                ColumnType::F32 | ColumnType::OptF32 => {
                    (input, opt) = parse_optional(column.typ, input, parser)?;
                    (input, value) = RawStream::from_bytes(input, parser)?;
                    properties.push(Raw(RP::F32(scalar(name, opt, value))));
                }
                ColumnType::F64 | ColumnType::OptF64 => {
                    (input, opt) = parse_optional(column.typ, input, parser)?;
                    (input, value) = RawStream::from_bytes(input, parser)?;
                    properties.push(Raw(RP::F64(scalar(name, opt, value))));
                }
                ColumnType::Str | ColumnType::OptStr => {
                    let prop;
                    (input, prop) = parse_str_column(input, name, column.typ, parser)?;
                    properties.push(Raw(prop));
                }
                ColumnType::SharedDict => {
                    let prop;
                    (input, prop) = parse_shared_dict_column(input, &column, parser)?;
                    properties.push(Raw(prop));
                }
            }
        }
        if input.is_empty() {
            Ok(Layer01 {
                name: layer_name,
                extent,
                id: id_column,
                geometry: geometry.ok_or(MissingGeometry)?,
                properties,
                #[cfg(fuzzing)]
                layer_order,
            })
        } else {
            Err(TrailingLayerData(input.len()))
        }
    }

    /// Decode only the ID column, leaving other columns in their encoded form.
    ///
    /// Use this instead of [`Self::decode_all`] when other columns will be accessed lazily.
    pub fn decode_id(&mut self, dec: &mut Decoder) -> MltResult<Option<&mut IdValues>> {
        Ok(if let Some(id) = &mut self.id {
            Some(id.decode(dec)?)
        } else {
            None
        })
    }

    /// Decode only the geometry column, leaving other columns in their encoded form.
    ///
    /// Use this instead of [`Self::decode_all`] when other columns will be accessed lazily.
    pub fn decode_geometry(&mut self, dec: &mut Decoder) -> MltResult<&mut GeometryValues> {
        self.geometry.decode(dec)
    }

    /// Decode only the property columns, leaving other columns in their encoded form.
    ///
    /// Use this instead of [`Self::decode_all`] when other columns will be accessed lazily.
    pub fn decode_properties(&mut self, dec: &mut Decoder) -> MltResult<()> {
        for prop in &mut self.properties {
            prop.decode(dec)?;
        }
        Ok(())
    }

    /// Decode all columns and transition to [`Layer01<Parsed>`].
    ///
    /// Consumes `self` (a `Layer01<Lazy>`) and returns a `Layer01<Parsed>` where every
    /// column field holds its parsed value directly, enabling infallible readonly access.
    pub fn decode_all(self, dec: &mut Decoder) -> MltResult<ParsedLayer01<'a>> {
        Ok(Layer01 {
            name: self.name,
            extent: self.extent,
            id: self.id.map(|id| id.into_parsed(dec)).transpose()?,
            geometry: self.geometry.into_parsed(dec)?,
            properties: self
                .properties
                .into_iter()
                .map(|p| p.into_parsed(dec))
                .collect::<MltResult<Vec<_>>>()?,
            #[cfg(fuzzing)]
            layer_order: self.layer_order,
        })
    }
}

fn parse_struct_children<'a>(
    mut input: &'a [u8],
    column: &Column<'a>,
    parser: &mut Parser,
) -> MltRefResult<'a, Vec<RawSharedDictItem<'a>>> {
    let mut children = Vec::with_capacity(column.children.len());
    for child in &column.children {
        let (inp, sc) = parse_varint::<u32>(input)?;
        let (inp, child_optional) = parse_optional(child.typ, inp, parser)?;
        let optional_stream_count = u32::from(child_optional.is_some());
        if let Some(data_count) = sc.checked_sub(optional_stream_count)
            && data_count != 1
        {
            return Err(UnexpectedStructChildCount(data_count));
        }
        let (inp, child_data) = RawStream::from_bytes(inp, parser)?;
        children.push(RawSharedDictItem {
            name: child.name.unwrap_or(""),
            presence: RawPresence(child_optional),
            data: child_data,
        });
        input = inp;
    }
    Ok((input, children))
}

fn parse_optional<'a>(
    typ: ColumnType,
    input: &'a [u8],
    parser: &mut Parser,
) -> MltRefResult<'a, Option<RawStream<'a>>> {
    if typ.is_optional() {
        let (input, optional) = RawStream::parse_bool(input, parser)?;
        Ok((input, Some(optional)))
    } else {
        Ok((input, None))
    }
}

fn parse_geometry_column<'a>(
    input: &'a [u8],
    geometry: &mut Option<Geometry<'a>>,
    parser: &mut Parser,
) -> MltResult<&'a [u8]> {
    let (input, stream_count) = parse_varint::<u32>(input)?;
    if stream_count == 0 {
        return Err(GeometryWithoutStreams);
    }
    // Each stream requires at least 1 byte (physical stream type)
    let stream_count_capa = stream_count.as_usize();
    if input.len() < stream_count_capa {
        return Err(BufferUnderflow(stream_count, input.len()));
    }
    // metadata
    let (input, meta) = RawStream::from_bytes(input, parser)?;
    // geometry items
    let (input, items) = RawStream::parse_multiple(input, stream_count_capa - 1, parser)?;
    geometry.set_once(Raw(RawGeometry { meta, items }))?;
    Ok(input)
}

fn parse_str_column<'a>(
    mut input: &'a [u8],
    name: &'a str,
    typ: ColumnType,
    parser: &mut Parser,
) -> MltRefResult<'a, RawProperty<'a>> {
    let mut stream_count = {
        let stream_count_u32;
        (input, stream_count_u32) = parse_varint::<u32>(input)?;
        stream_count_u32.as_usize()
    };
    let presence;
    (input, presence) = parse_optional(typ, input, parser)?;
    if presence.is_some() {
        if stream_count == 0 {
            return Err(UnsupportedStringStreamCount(stream_count));
        }
        stream_count -= 1;
    }
    let mut str_streams = [None, None, None, None, None];
    if stream_count > str_streams.len() {
        return Err(UnsupportedStringStreamCount(stream_count));
    }
    for slot in str_streams.iter_mut().take(stream_count) {
        let stream;
        (input, stream) = RawStream::from_bytes(input, parser)?;
        *slot = Some(stream);
    }
    let encoding = match str_streams {
        [Some(s1), Some(s2), None, None, None] => {
            RawStringsEncoding::plain(RawPlainData::new(s1, s2)?)
        }
        [Some(s1), Some(s2), Some(s3), None, None] => {
            RawStringsEncoding::dictionary(RawPlainData::new(s1, s3)?, s2)?
        }
        [Some(s1), Some(s2), Some(s3), Some(s4), None] => {
            RawStringsEncoding::fsst_plain(RawFsstData::new(s1, s2, s3, s4)?)
        }
        [Some(s1), Some(s2), Some(s3), Some(s4), Some(s5)] => {
            RawStringsEncoding::fsst_dictionary(RawFsstData::new(s1, s2, s3, s4)?, s5)?
        }
        _ => Err(UnsupportedStringStreamCount(stream_count))?,
    };
    Ok((
        input,
        RawProperty::Str(RawStrings {
            name,
            presence: RawPresence(presence),
            encoding,
        }),
    ))
}

fn parse_shared_dict_column<'a>(
    mut input: &'a [u8],
    column: &Column<'a>,
    parser: &mut Parser,
) -> MltRefResult<'a, RawProperty<'a>> {
    // Read header streams until we hit the dictionary DATA(Single|Shared) stream.
    let stream_count;
    (input, stream_count) = parse_varint::<u32>(input)?;
    let mut dict_streams = [None, None, None, None, None];
    let mut streams_taken = 0_usize;
    while streams_taken < stream_count.as_usize() {
        let stream;
        (input, stream) = RawStream::from_bytes(input, parser)?;
        let is_last = matches!(
            stream.meta.stream_type,
            StreamType::Data(DictionaryType::Single | DictionaryType::Shared)
        );
        dict_streams[streams_taken] = Some(stream);
        streams_taken += 1;
        if is_last {
            break;
        } else if streams_taken >= dict_streams.len() {
            return Err(UnsupportedStringStreamCount(streams_taken + 1));
        }
    }
    let children;
    (input, children) = parse_struct_children(input, column, parser)?;

    // Validate stream_count: must equal dict_streams + children + optional_children.
    let children_n = u32::try_from(children.len()).or_overflow()?;
    let optional_n = children
        .iter()
        .filter(|c| c.presence.0.is_some())
        .count()
        .try_into()
        .or_overflow()?;
    let dict_n = u32::try_from(streams_taken).or_overflow()?;
    let expected = crate::utils::checked_sum3(dict_n, children_n, optional_n)?;
    // Java's encoder had a bug (fixed) that overcounted by 1: dict + 2*N + 1.
    // Accept that value too so that files produced by older Java encoders still parse.
    let java_legacy = expected.checked_add(1).or_overflow()?;
    if stream_count != expected && stream_count != java_legacy {
        return Err(InvalidSharedDictStreamCount {
            actual: stream_count,
            expected,
        });
    }

    let name = column.name.unwrap_or("");
    let encoding = match dict_streams {
        [Some(s1), Some(s2), None, None, None] => {
            RawSharedDictEncoding::plain(RawPlainData::new(s1, s2)?)
        }
        [Some(s1), Some(s2), Some(s3), Some(s4), None] => {
            RawSharedDictEncoding::fsst_plain(RawFsstData::new(s1, s2, s3, s4)?)
        }
        _ => Err(SharedDictRequiresStreams(streams_taken))?,
    };
    Ok((
        input,
        RawProperty::SharedDict(RawSharedDict {
            name,
            encoding,
            children,
        }),
    ))
}

fn parse_columns_meta<'a>(
    mut input: &'a [u8],
    column_count: u32,
    parser: &mut Parser,
) -> MltRefResult<'a, (Vec<Column<'a>>, u32)> {
    use crate::decoder::ColumnType::{Geometry, Id, LongId, OptId, OptLongId, SharedDict};

    let mut col_info = Vec::with_capacity(column_count.as_usize());
    let mut geometries = 0;
    let mut ids = 0;
    for _ in 0..column_count {
        let mut typ;
        (input, typ) = Column::from_bytes(input, parser)?;
        match typ.typ {
            Geometry => geometries += 1,
            Id | OptId | LongId | OptLongId => ids += 1,
            SharedDict => {
                // Yes, we need to parse children right here; otherwise this messes up the next column
                let child_column_count;
                (input, child_column_count) = parse_varint::<u32>(input)?;

                // Each column requires at least 1 byte (ColumnType without a name)
                let child_col_capacity = child_column_count.as_usize();
                if input.len() < child_col_capacity {
                    return Err(BufferUnderflow(child_column_count, input.len()));
                }
                let mut children = Vec::with_capacity(child_col_capacity);
                for _ in 0..child_column_count {
                    let child;
                    (input, child) = Column::from_bytes(input, parser)?;
                    children.push(child);
                }
                typ.children = children;
            }
            _ => {}
        }
        col_info.push(typ);
    }
    if geometries > 1 {
        return Err(MultipleGeometryColumns);
    }
    if ids > 1 {
        return Err(MultipleIdColumns);
    }

    Ok((input, (col_info, column_count - geometries - ids)))
}

fn scalar<'a>(name: &'a str, opt: Option<RawStream<'a>>, value: RawStream<'a>) -> RawScalar<'a> {
    RawScalar {
        name,
        presence: RawPresence(opt),
        data: value,
    }
}
