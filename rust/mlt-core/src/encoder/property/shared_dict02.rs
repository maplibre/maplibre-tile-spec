//! Writer for a v2 shared-dictionary column: one dictionary, then the columns that index into it.

use std::collections::HashMap;

use integer_encoding::VarIntWriter as _;
use usize_cast::IntoUsize as _;

use crate::MltError::DictIndexOutOfBounds;
use crate::codecs::front_coding::front_code;
use crate::codecs::fsst::{compress_fsst_bytes, compress_fsst_with};
use crate::decoder::stream::header02::{BlobLayout, Family, StrLayout};
use crate::decoder::{ColumnType02, DataType02, Presence02, SharedDictKind};
use crate::encoder::encode02::write_presence_bits;
use crate::encoder::model::StreamCtx;
use crate::encoder::property::shared_dict::collect_staged_shared_dict_spans;
use crate::encoder::property::strings::{
    suffix_parts, write_blob02, write_dict_tail02, write_front_lengths02, write_fsst_tail02,
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
    /// dictionary front coded or not. Front coding needs the entries sorted, which renumbers every
    /// child's codes, so both orderings are built up front.
    #[hotpath::measure]
    pub(crate) fn write_shared_dict02(
        &mut self,
        shared_dict: &StagedSharedDict,
        enc: &mut Encoder,
    ) -> MltResult<()> {
        let plain = group(shared_dict)?;
        let mut sorted_entries = plain.entries.clone();
        sorted_entries.sort_unstable();
        let sorted = resort(&plain, &sorted_entries)?;
        let front = front_code(&sorted.entries)?;

        // FSST is trained on what it compresses: the entries for a plain dictionary,
        // the suffixes for a front-coded one.
        let compressor = enc.fsst_compressor(&shared_dict.prefix, &plain.entries);
        let fsst = compressor.map(|c| compress_fsst_with(&plain.entries, c));
        let front_fsst = compressor.map(|_| {
            let parts = suffix_parts(&front);
            compress_fsst_bytes(&parts, &front.suffixes)
        });

        let name = &shared_dict.prefix;
        // The dictionary's entry count is nothing the decoder can imply, so its streams are
        // written against an implicit count of zero, which is what makes them carry their own.
        let features = enc.count_context;
        let mut alt = enc.try_alternatives();
        alt.with(|enc| {
            begin_shared_dict02(enc, SharedDictKind::Plain, shared_dict)?;
            write_dict_tail02(&plain.entries, name, enc, self)?;
            write_children02(shared_dict, &plain.codes, features, enc, self)
        })?;
        alt.with(|enc| {
            begin_shared_dict02(enc, SharedDictKind::Plain, shared_dict)?;
            write_front_lengths02(&front, name, enc, self)?;
            write_blob02(&front.suffixes, BlobLayout::FrontCoded, enc)?;
            write_children02(shared_dict, &sorted.codes, features, enc, self)
        })?;
        if let Some(ref raw) = fsst {
            alt.with(|enc| {
                begin_shared_dict02(enc, SharedDictKind::Fsst, shared_dict)?;
                let ctx = StreamCtx::prop(StreamType::Length(LengthType::Dictionary), name);
                self.write_int_stream(&raw.value_lengths, &ctx, enc)?;
                write_fsst_tail02(
                    &raw.symbol_lengths,
                    &raw.symbol_bytes,
                    &raw.corpus,
                    BlobLayout::Plain,
                    name,
                    enc,
                    self,
                )?;
                write_children02(shared_dict, &plain.codes, features, enc, self)
            })?;
        }
        if let Some(ref blob) = front_fsst {
            alt.with(|enc| {
                begin_shared_dict02(enc, SharedDictKind::Fsst, shared_dict)?;
                write_front_lengths02(&front, name, enc, self)?;
                write_fsst_tail02(
                    &blob.symbol_lengths,
                    &blob.symbol_bytes,
                    &blob.corpus,
                    BlobLayout::FrontCoded,
                    name,
                    enc,
                    self,
                )?;
                write_children02(shared_dict, &sorted.codes, features, enc, self)
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

/// The same dictionary in lexicographic order, with every child's codes renumbered to match.
fn resort<'a>(plain: &Grouped<'a>, sorted_entries: &[&'a str]) -> MltResult<Grouped<'a>> {
    let mut rank: HashMap<&str, u32> = HashMap::with_capacity(sorted_entries.len());
    for (new, &entry) in sorted_entries.iter().enumerate() {
        rank.insert(entry, u32::try_from(new)?);
    }
    let codes = plain
        .codes
        .iter()
        .map(|child| {
            child
                .iter()
                .map(|&code| {
                    let entry = plain.entries[code.into_usize()];
                    rank.get(entry)
                        .copied()
                        .ok_or(DictIndexOutOfBounds(code, sorted_entries.len()))
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<_, _>>()?;
    Ok(Grouped {
        entries: sorted_entries.to_vec(),
        codes,
    })
}

/// Write the column's type byte, whose high nibble names `kind`, its name and its child count.
fn begin_shared_dict02(
    enc: &mut Encoder,
    kind: SharedDictKind,
    shared_dict: &StagedSharedDict,
) -> MltResult<()> {
    enc.family_context = Family::Int;
    enc.count_context = 0;
    let byte = kind as u8 | DataType02::SharedDict as u8;
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
    features: u32,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    for (item, child_codes) in shared_dict.items.iter().zip(per_child_codes) {
        let presence: Vec<bool> = item.presence_bools().collect();
        let optional = item.has_presence() && presence.iter().any(|&p| !p);
        let where_ = if optional {
            Presence02::Inline
        } else {
            Presence02::AllPresent
        };
        let data = enc.data_mut();
        data.push(ColumnType02::new(where_, DataType02::Str).to_byte());
        data.write_string(&item.suffix)?;
        if optional {
            write_presence_bits(enc.data_mut(), &presence);
        }

        enc.count_context = u32::try_from(child_codes.len())?;
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
