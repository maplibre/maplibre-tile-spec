//! Layer envelope and column writers for tag `0x02` (v2) layers.
//!
//! A v2 layer body is: header (`name`, `extent`, `feature_count`), geometry
//! section, `column_count` varint, then each counted column as
//! `[type byte][name?][presence bitfield?][data stream]` — metadata and data
//! merged, unlike v1's split sections.
//!
//! Stream payload encodings (and their size competitions) are shared with v1
//! via [`Codecs::write_int_stream`]; only the envelope, presence
//! representation, and stream headers differ.

use integer_encoding::VarIntWriter as _;

use crate::decoder::{ColumnType02, DictionaryType, StreamType};
use crate::encoder::geometry::encode02::write_geometry02;
use crate::encoder::model::{StagedLayer, StreamCtx};
use crate::encoder::{Codecs, Encoder, StagedId, StagedProperty};
use crate::utils::BinarySerializer as _;
use crate::{MltError, MltResult};

/// Encode and serialize a staged layer as a v2 (tag `0x02`) body into `enc`.
///
/// The v2 counterpart of the v1 `StagedLayer::encode_into` body; dispatched
/// from there based on [`EncoderConfig::wire_version`](crate::encoder::EncoderConfig::wire_version).
pub(crate) fn encode_into02(
    layer: StagedLayer,
    mut enc: Encoder,
    codecs: &mut Codecs,
) -> MltResult<Encoder> {
    let StagedLayer {
        name,
        extent,
        id,
        geometry,
        properties,
    } = layer;

    let feature_count = u32::try_from(geometry.feature_count())?;
    enc.count_context = feature_count;

    // ── Geometry section (not part of column_count) ───────────────────────
    write_geometry02(geometry, &mut enc, codecs)?;

    // ── Counted columns ───────────────────────────────────────────────────
    let column_count = usize::from(!matches!(id, StagedId::None)) + properties.len();
    enc.data_mut().write_varint(u32::try_from(column_count)?)?;

    write_id02(id, &mut enc, codecs)?;
    for prop in &properties {
        write_prop02(prop, &mut enc, codecs)?;
    }

    enc.write_header02(&name, extent.get(), feature_count)?;
    Ok(enc)
}

/// Write a column's type byte and, for named columns, its name — inline in the
/// data section (v2 has no separate metadata section).
fn begin_col02(enc: &mut Encoder, typ: ColumnType02, name: Option<&str>) -> MltResult<()> {
    let data = enc.data_mut();
    data.push(typ as u8);
    debug_assert_eq!(typ.has_name(), name.is_some());
    if let Some(name) = name {
        data.write_string(name)?;
    }
    Ok(())
}

/// Write a raw packed presence bitfield (`ceil(len/8)` bytes, LSB-first),
/// then run `write_data` with [`Encoder::count_context`] set to the presence
/// popcount — the implicit count of the optional column's data stream.
fn write_opt_col02<F>(enc: &mut Encoder, presence: &[bool], write_data: F) -> MltResult<()>
where
    F: FnOnce(&mut Encoder) -> MltResult<()>,
{
    let data = enc.data_mut();
    let start = data.len();
    data.resize(start + presence.len().div_ceil(8), 0);
    let mut popcount: u32 = 0;
    for (i, &present) in presence.iter().enumerate() {
        if present {
            data[start + i / 8] |= 1 << (i % 8);
            popcount += 1;
        }
    }

    let feature_count = enc.count_context;
    enc.count_context = popcount;
    let result = write_data(enc);
    enc.count_context = feature_count;
    result
}

fn write_id02(id: StagedId, enc: &mut Encoder, codecs: &mut Codecs) -> MltResult<()> {
    use ColumnType02 as CT;
    let ctx = StreamCtx::id(StreamType::Data(DictionaryType::None));
    match id {
        StagedId::None => Ok(()),
        StagedId::U32(v) => {
            begin_col02(enc, CT::Id, None)?;
            codecs.write_int_stream(&v.values, &ctx, enc)
        }
        StagedId::OptU32(v) => {
            begin_col02(enc, CT::OptId, None)?;
            write_opt_col02(enc, &v.presence, |enc| {
                codecs.write_int_stream(&v.values, &ctx, enc)
            })
        }
        StagedId::U64(v) => {
            begin_col02(enc, CT::LongId, None)?;
            codecs.write_int_stream(&v.values, &ctx, enc)
        }
        StagedId::OptU64(v) => {
            begin_col02(enc, CT::OptLongId, None)?;
            write_opt_col02(enc, &v.presence, |enc| {
                codecs.write_int_stream(&v.values, &ctx, enc)
            })
        }
    }
}

/// Encode a single property column, dispatching on variant.
///
/// The v2 counterpart of the v1 `write_prop`: the column header goes inline
/// into the data section, presence is a raw bitfield, and bool data is written
/// as an ordinary 0/1 integer stream (racing raw vs RLE) instead of v1's
/// special bool-RLE bitset.
fn write_prop02(prop: &StagedProperty, enc: &mut Encoder, codecs: &mut Codecs) -> MltResult<()> {
    use ColumnType02 as CT;
    use StagedProperty as D;

    /// Non-optional scalar: `[type][name][data stream]`.
    macro_rules! scalar {
        ($ct:ident, $v:expr) => {{
            begin_col02(enc, CT::$ct, Some(&$v.name))?;
            codecs.write_int_stream(&$v.values, &StreamCtx::prop_data(&$v.name), enc)
        }};
    }
    /// Optional scalar: `[type][name][presence bitfield][data stream]`.
    macro_rules! opt_scalar {
        ($ct:ident, $v:expr) => {{
            begin_col02(enc, CT::$ct, Some(&$v.name))?;
            write_opt_col02(enc, &$v.presence, |enc| {
                codecs.write_int_stream(&$v.values, &StreamCtx::prop_data(&$v.name), enc)
            })
        }};
    }

    match prop {
        D::Bool(v) => {
            begin_col02(enc, CT::Bool, Some(&v.name))?;
            let bytes: Vec<u8> = v.values.iter().copied().map(u8::from).collect();
            codecs.write_int_stream(&bytes, &StreamCtx::prop_data(&v.name), enc)
        }
        D::OptBool(v) => {
            begin_col02(enc, CT::OptBool, Some(&v.name))?;
            let bytes: Vec<u8> = v.values.iter().copied().map(u8::from).collect();
            write_opt_col02(enc, &v.presence, |enc| {
                codecs.write_int_stream(&bytes, &StreamCtx::prop_data(&v.name), enc)
            })
        }
        D::F32(v) => {
            begin_col02(enc, CT::F32, Some(&v.name))?;
            codecs.write_float_stream(&v.values, StreamType::Data(DictionaryType::None), enc)
        }
        D::OptF32(v) => {
            begin_col02(enc, CT::OptF32, Some(&v.name))?;
            write_opt_col02(enc, &v.presence, |enc| {
                codecs.write_float_stream(&v.values, StreamType::Data(DictionaryType::None), enc)
            })
        }
        D::F64(v) => {
            begin_col02(enc, CT::F64, Some(&v.name))?;
            codecs.write_float_stream(&v.values, StreamType::Data(DictionaryType::None), enc)
        }
        D::OptF64(v) => {
            begin_col02(enc, CT::OptF64, Some(&v.name))?;
            write_opt_col02(enc, &v.presence, |enc| {
                codecs.write_float_stream(&v.values, StreamType::Data(DictionaryType::None), enc)
            })
        }
        D::I8(v) => scalar!(I8, v),
        D::OptI8(v) => opt_scalar!(OptI8, v),
        D::U8(v) => scalar!(U8, v),
        D::OptU8(v) => opt_scalar!(OptU8, v),
        D::I32(v) => scalar!(I32, v),
        D::OptI32(v) => opt_scalar!(OptI32, v),
        D::U32(v) => scalar!(U32, v),
        D::OptU32(v) => opt_scalar!(OptU32, v),
        D::I64(v) => scalar!(I64, v),
        D::OptI64(v) => opt_scalar!(OptI64, v),
        D::U64(v) => scalar!(U64, v),
        D::OptU64(v) => opt_scalar!(OptU64, v),
        D::Str(_) | D::OptStr(_) => Err(MltError::NotImplemented("v2 string columns")),
        D::SharedDict(_) => Err(MltError::NotImplemented("v2 shared dictionary columns")),
    }
}
