//! Writer for a v2 shared-dictionary column: one dictionary, then the columns that index into it.

use std::collections::HashMap;

use integer_encoding::VarIntWriter as _;

use crate::MltError::DictIndexOutOfBounds;
use crate::codecs::front_coding::front_code;
use crate::codecs::fsst::{compress_fsst_bytes, compress_fsst_with};
use crate::decoder::stream::header02::{Count02, Family, StrLayout};
use crate::decoder::{Column02, ColumnType02, DataType02, DictLayout, Presence02, SharedDictKind};
use crate::encoder::encode02::{SharedPresence, write_presence_bits};
use crate::encoder::model::{StrAt, StreamCtx};
use crate::encoder::property::shared_dict::collect_staged_shared_dict_spans;
use crate::encoder::property::strings::{
    recode, sort_dictionary, suffix_parts, write_blob02, write_dict_tail02, write_front_lengths02,
    write_fsst_tail02,
};
use crate::encoder::{Codecs, Encoder, StagedSharedDict};
use crate::utils::BinarySerializer as _;
use crate::{LengthType, MltResult, OffsetType, StreamType};

/// The dictionary a shared-dictionary column writes, with each child's codes into it.
struct Grouped<'a> {
    /// Distinct entries in the order their codes number them.
    entries: Vec<&'a str>,
    /// One code list per child, in child order, over that child's present values only.
    codes: Vec<Vec<u32>>,
}

impl Codecs {
    /// Encode a shared-dictionary property as a v2 column and write it to `enc`.
    ///
    /// Four shapes are raced on stored bytes: the corpus plain or FSST-compressed, each with the
    /// dictionary front coded or not.
    #[hotpath::measure]
    pub(crate) fn write_shared_dict02(
        &mut self,
        shared_dict: &StagedSharedDict,
        shared: &SharedPresence,
        enc: &mut Encoder,
    ) -> MltResult<()> {
        let plain = group(shared_dict)?;
        // Front coding needs the entries sorted, which renumbers every child's codes with them.
        let (sorted_entries, rank) = sort_dictionary(&plain.entries)?;
        let sorted_codes: Vec<Vec<u32>> = plain.codes.iter().map(|c| recode(c, &rank)).collect();
        let front = front_code(&sorted_entries)?;

        // FSST is trained on what it compresses: the entries for a plain dictionary,
        // the suffixes for a front-coded one.
        let compressor = enc.fsst_compressor(&shared_dict.prefix, &plain.entries);
        let fsst = compressor.map(|c| compress_fsst_with(&plain.entries, c));
        let front_fsst = enc.config().allow_fsst().then(|| {
            let parts = suffix_parts(&front);
            compress_fsst_bytes(&parts, &front.suffixes)
        });

        let name = &shared_dict.prefix;
        let features = enc.count_context;
        let mut alt = enc.try_alternatives();
        // Each shape writes the column's type byte, the tail its corpus is stored as, then the children.
        alt.with(|enc| {
            begin_shared_dict02(enc, SharedDictKind::Plain, shared_dict)?;
            write_dict_tail02(&plain.entries, StrAt::flat(name), enc, self)?;
            write_children02(shared_dict, &plain.codes, features, shared, enc, self)
        })?;
        alt.with(|enc| {
            begin_shared_dict02(enc, SharedDictKind::Plain, shared_dict)?;
            write_front_lengths02(&front, StrAt::flat(name), enc, self)?;
            write_blob02(&front.suffixes, DictLayout::FrontCoded, enc)?;
            write_children02(shared_dict, &sorted_codes, features, shared, enc, self)
        })?;
        if let Some(ref raw) = fsst {
            alt.with(|enc| {
                begin_shared_dict02(enc, SharedDictKind::Fsst, shared_dict)?;
                let ctx = StreamCtx::prop(StreamType::Length(LengthType::Dictionary), name);
                self.write_int_stream(&raw.value_lengths, &ctx, enc)?;
                write_fsst_tail02(&raw.blob, DictLayout::Plain, StrAt::flat(name), enc, self)?;
                write_children02(shared_dict, &plain.codes, features, shared, enc, self)
            })?;
        }
        if let Some(ref blob) = front_fsst {
            alt.with(|enc| {
                begin_shared_dict02(enc, SharedDictKind::Fsst, shared_dict)?;
                write_front_lengths02(&front, StrAt::flat(name), enc, self)?;
                write_fsst_tail02(blob, DictLayout::FrontCoded, StrAt::flat(name), enc, self)?;
                write_children02(shared_dict, &sorted_codes, features, shared, enc, self)
            })?;
        }
        Ok(())
    }
}

/// The distinct entries every child shares, and each child's codes into them.
fn group(shared_dict: &StagedSharedDict) -> MltResult<Grouped<'_>> {
    let spans = collect_staged_shared_dict_spans(&shared_dict.items);
    let entries: Vec<&str> = spans
        .iter()
        .map(|&span| {
            shared_dict
                .get(span)
                .ok_or(DictIndexOutOfBounds(span.0, spans.len()))
        })
        .collect::<Result<_, _>>()?;
    let index: HashMap<(u32, u32), u32> = spans.iter().copied().zip(0_u32..).collect();

    let codes = shared_dict
        .items
        .iter()
        .map(|item| {
            item.dense_spans()
                .map(|span| {
                    index
                        .get(&span)
                        .copied()
                        .ok_or(DictIndexOutOfBounds(span.0, spans.len()))
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<_, _>>()?;
    Ok(Grouped { entries, codes })
}

/// Write the column's type byte, whose high nibble names `kind`, its name and its child count.
///
/// The dictionary's entry count is nothing the decoder can infer, so its streams are
/// written against no implied count, which is what makes each carry its own.
fn begin_shared_dict02(
    enc: &mut Encoder,
    kind: SharedDictKind,
    shared_dict: &StagedSharedDict,
) -> MltResult<()> {
    enc.family_context = Family::Int;
    enc.count_context = Count02::Explicit;
    let byte = kind as u8 | Column02::SHARED_DICT;
    let data = enc.data_mut();
    data.push(byte);
    data.write_string(&shared_dict.prefix)?;
    data.write_varint(u32::try_from(shared_dict.items.len())?)?;
    Ok(())
}

/// Write each child: its type byte and name, its presence bitfield, then its codes.
fn write_children02(
    shared_dict: &StagedSharedDict,
    per_child_codes: &[Vec<u32>],
    features: Count02,
    shared: &SharedPresence,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    for (item, child_codes) in shared_dict.items.iter().zip(per_child_codes) {
        let presence = item.optional_presence();
        let where_ = presence
            .as_ref()
            .map_or(Presence02::AllPresent, |mask| shared.nibble_for(mask));
        let data = enc.data_mut();
        data.push(ColumnType02::new(where_, DataType02::Str).to_byte());
        data.write_string(&item.suffix)?;
        if let (Presence02::Inline, Some(mask)) = (where_, &presence) {
            write_presence_bits(enc.data_mut(), mask);
        }

        enc.count_context = Count02::Implied(u32::try_from(child_codes.len())?);
        let ctx = StreamCtx::prop2(
            StreamType::Offset(OffsetType::String),
            &shared_dict.prefix,
            &item.suffix,
        );
        enc.family_context = Family::Str(StrLayout::Dict);
        let result = codecs.write_int_stream(child_codes, &ctx, enc);
        enc.family_context = Family::Int;
        result?;
    }
    // Restore what the enclosing layer's remaining columns imply their counts from.
    enc.count_context = features;
    Ok(())
}
