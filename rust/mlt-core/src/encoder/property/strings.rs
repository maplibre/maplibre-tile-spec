use fsst::Compressor;
use integer_encoding::VarIntWriter as _;
use usize_cast::IntoUsize as _;

use super::model::StagedStrings;
use crate::MltResult;
#[cfg(feature = "unstable-v2")]
use crate::codecs::front_coding::{FrontCoded, front_code};
#[cfg(feature = "unstable-v2")]
use crate::codecs::fsst::{FsstBlob, compress_fsst_bytes};
use crate::codecs::fsst::{FsstRawData, compress_fsst, compress_fsst_with};
use crate::decoder::stream::header01;
#[cfg(feature = "unstable-v2")]
use crate::decoder::stream::header02;
#[cfg(feature = "unstable-v2")]
use crate::decoder::stream::header02::{BlobLayout, Family, StrLayout, StreamCtx02};
use crate::decoder::strings::{checked_string_end, encode_null_end};
use crate::decoder::{DictionaryType, LengthType, OffsetType, StreamMeta, StreamType, ValueKind};
use crate::encoder::model::{StrEncoding, StreamCtx};
use crate::encoder::stream::{dedup_strings, write_stream_payload};
use crate::encoder::{Codecs, Encoder};
use crate::utils::strings_to_lengths;

/// Minimum total raw byte size of a column before attempting FSST compression.
const FSST_OVERHEAD_THRESHOLD: usize = 2_048;
/// Maximum number of strings sampled for the FSST viability probe.
const FSST_SAMPLE_STRINGS: usize = 256;

/// Train an FSST compressor and return it when compression is likely to save space.
///
/// Returns `None` when the column is empty, too small for FSST overhead to pay off,
/// or when trial compression shows no benefit.
///
/// Training always uses all values so the symbol table sees the full distribution.
/// The viability probe (trial compression) is limited to [`FSST_SAMPLE_STRINGS`] to
/// bound cost.
#[hotpath::measure]
pub(crate) fn fsst_try_train(strings: &[&str]) -> Option<Compressor> {
    if strings.is_empty() {
        return None;
    }
    let total_plain_size: usize = strings.iter().map(|s| s.len()).sum();
    if total_plain_size < FSST_OVERHEAD_THRESHOLD {
        return None;
    }
    let byte_slices: Vec<&[u8]> = strings.iter().map(|s| s.as_bytes()).collect();
    let compressor = Compressor::train(&byte_slices);
    let symbols = compressor.symbol_table();
    let symbol_lengths = compressor.symbol_lengths();
    let symbol_overhead: usize = symbol_lengths
        .iter()
        .take(symbols.len())
        .map(|&l| usize::from(l))
        .sum();
    let sample = if strings.len() <= FSST_SAMPLE_STRINGS {
        strings
    } else {
        &strings[..FSST_SAMPLE_STRINGS]
    };
    let plain_size: usize = sample.iter().map(|s| s.len()).sum();
    let compressed_size: usize = sample
        .iter()
        .map(|s| compressor.compress(s.as_bytes()).len())
        .sum();
    if symbol_overhead + compressed_size < plain_size {
        Some(compressor)
    } else {
        None
    }
}

impl Encoder {
    /// FSST compressor for a column corpus, trained and cached under `key` on first use.
    /// Returns `None` if FSST is disabled ([`EncoderConfig::allow_fsst`]) or not worthwhile for `corpus`.
    /// The single gate for FSST in the auto path; explicit encodings bypass it.
    ///
    /// [`EncoderConfig::allow_fsst`]: crate::encoder::EncoderConfig::allow_fsst
    pub(crate) fn fsst_compressor(&mut self, key: &str, corpus: &[&str]) -> Option<&Compressor> {
        if !self.config().allow_fsst() {
            return None;
        }
        self.fsst_cache
            .entry(key.to_owned())
            .or_insert_with(|| fsst_try_train(corpus))
            .as_ref()
    }
}

impl Codecs {
    /// Encode a string column, following the same explicit-or-auto pattern as numeric columns.
    ///
    /// If [`Encoder::override_str_enc`] returns `Some`, only that type is encoded.
    /// Otherwise Plain, Dict, and (when viable) FSST variants are competed via the alternatives
    /// machinery, mirroring the `write_int_prop_*` pattern one level up.
    #[hotpath::measure]
    pub(crate) fn write_str_col(
        &mut self,
        v: &StagedStrings,
        presence: Option<&StagedStrings>,
        enc: &mut Encoder,
    ) -> MltResult<()> {
        let non_null = v.dense_values();
        let name = &v.name;
        if let Some(str_enc) = enc.override_str_enc(name) {
            match str_enc {
                StrEncoding::Plain => write_str_plain(&non_null, presence, name, enc, self)?,
                StrEncoding::Dict => write_str_dict(&non_null, presence, name, enc, self)?,
                StrEncoding::Fsst => write_str_fsst(&non_null, presence, name, enc, self)?,
                StrEncoding::FsstDict => write_str_fsst_dict(&non_null, presence, name, enc, self)?,
            }
        } else {
            // Dedup once; reused by Dict and FSST+Dict alternatives.
            let (unique, offset_indices) = dedup_strings(&non_null)?;

            // `None` disables FSST, so only Plain and Dict compete.
            let compressor = enc.fsst_compressor(name, &unique);

            // Compute before try_alternatives borrows enc; FsstRawData is owned so the cache borrow ends here.
            let count = non_null.len();
            let plain_fsst = compressor.map(|c| compress_fsst_with(&non_null, c));
            let dict_fsst = compressor.map(|c| compress_fsst_with(&unique, c));

            let mut alt = enc.try_alternatives();
            alt.with(|enc| write_str_plain(&non_null, presence, name, enc, self))?;
            alt.with(|enc| {
                write_str_dict_raw(&unique, &offset_indices, presence, name, enc, self)
            })?;

            if let Some(ref raw) = plain_fsst {
                alt.with(|enc| write_str_fsst_raw(raw, count, presence, name, enc, self))?;
            }
            if let Some(ref raw) = dict_fsst {
                alt.with(|enc| {
                    write_str_fsst_dict_raw(raw, &offset_indices, presence, name, enc, self)
                })?;
            }
        }
        Ok(())
    }
}

#[cfg(feature = "unstable-v2")]
impl Codecs {
    /// Encode a v2 string column as the [`StrLayout`] that stores the fewest bytes.
    ///
    /// Mirrors [`Self::write_str_col`]: [`Encoder::override_str_enc`] pins one layout when set,
    /// otherwise all four compete. Nulls live in the column's presence bitfield rather than in a
    /// stream of this column's own, so every layout writes only the present values.
    #[hotpath::measure]
    pub(crate) fn write_str_col02(
        &mut self,
        v: &StagedStrings,
        enc: &mut Encoder,
    ) -> MltResult<()> {
        let non_null = v.dense_values();
        let name = &v.name;
        if let Some(str_enc) = enc.override_str_enc(name) {
            return match str_enc {
                StrEncoding::Plain => write_str_plain02(&non_null, name, enc, self),
                StrEncoding::Dict => {
                    let (unique, codes) = dedup_strings(&non_null)?;
                    write_str_dict02(&unique, &codes, name, enc, self)
                }
                StrEncoding::Fsst => write_str_fsst02(&compress_fsst(&non_null), name, enc, self),
                StrEncoding::FsstDict => {
                    let (unique, codes) = dedup_strings(&non_null)?;
                    write_str_fsst_dict02(&compress_fsst(&unique), &codes, name, enc, self)
                }
            };
        }

        // Dedup once; reused by Dict and FSST+Dict alternatives.
        let (unique, codes) = dedup_strings(&non_null)?;
        // Front coding needs the dictionary sorted, which renumbers the codes with it.
        let (sorted, sorted_codes) = sort_dictionary(&unique, &codes)?;
        let front = front_code(&sorted)?;
        // `None` disables FSST, so only Plain and Dict compete.
        let compressor = enc.fsst_compressor(name, &unique);
        // Compute before try_alternatives borrows enc; FsstRawData is owned so the cache borrow ends here.
        let plain_fsst = compressor.map(|c| compress_fsst_with(&non_null, c));
        let dict_fsst = compressor.map(|c| compress_fsst_with(&unique, c));
        // FSST over the suffixes is trained on them, since they are not the corpus `unique` is.
        let front_fsst = compressor.map(|_| {
            let parts = suffix_parts(&front);
            compress_fsst_bytes(&parts, &front.suffixes)
        });

        let mut alt = enc.try_alternatives();
        alt.with(|enc| write_str_plain02(&non_null, name, enc, self))?;
        alt.with(|enc| write_str_dict02(&unique, &codes, name, enc, self))?;
        alt.with(|enc| write_str_front_dict02(&front, &sorted_codes, name, enc, self))?;
        if let Some(ref raw) = plain_fsst {
            alt.with(|enc| write_str_fsst02(raw, name, enc, self))?;
        }
        if let Some(ref raw) = dict_fsst {
            alt.with(|enc| write_str_fsst_dict02(raw, &codes, name, enc, self))?;
        }
        if let Some(ref raw) = front_fsst {
            alt.with(|enc| {
                write_str_fsst_front_dict02(&front, raw, &sorted_codes, name, enc, self)
            })?;
        }
        Ok(())
    }
}

/// Write a v2 string column's leading stream, whose extension bits name the layout the rest follow.
#[cfg(feature = "unstable-v2")]
fn write_str_leading02(
    values: &[u32],
    layout: StrLayout,
    name: &str,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    let ctx = StreamCtx::prop(StreamCtx02::StrData(layout).stream_type(), name);
    enc.family_context = Family::Str(layout);
    let result = codecs.write_int_stream(values, &ctx, enc);
    enc.family_context = Family::Int;
    result
}

/// Write already-encoded bytes as a v2 blob, whose byte length is also its value count.
#[cfg(feature = "unstable-v2")]
fn write_blob02(bytes: &[u8], layout: BlobLayout, enc: &mut Encoder) -> MltResult<()> {
    header02::write_blob_meta(enc.data_mut(), layout, u32::try_from(bytes.len())?)?;
    enc.data_mut().extend_from_slice(bytes);
    Ok(())
}

/// Write `strings` back to back as a v2 blob.
#[cfg(feature = "unstable-v2")]
fn write_str_blob02(strings: &[&str], enc: &mut Encoder) -> MltResult<()> {
    let total: usize = strings.iter().map(|s| s.len()).sum();
    header02::write_blob_meta(enc.data_mut(), BlobLayout::Plain, u32::try_from(total)?)?;
    let data = enc.data_mut();
    data.reserve(total);
    for s in strings {
        data.extend_from_slice(s.as_bytes());
    }
    Ok(())
}

/// [`StrLayout::Plain`]: one length per value, then the values' bytes.
#[cfg(feature = "unstable-v2")]
fn write_str_plain02(
    non_null: &[&str],
    name: &str,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    let lengths = strings_to_lengths(non_null)?;
    write_str_leading02(&lengths, StrLayout::Plain, name, enc, codecs)?;
    write_str_blob02(non_null, enc)
}

/// [`StrLayout::Dict`]: one code per value, then the distinct values' lengths and bytes.
#[cfg(feature = "unstable-v2")]
fn write_str_dict02(
    unique: &[&str],
    offset_indices: &[u32],
    name: &str,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    write_str_leading02(offset_indices, StrLayout::Dict, name, enc, codecs)?;
    let lengths = strings_to_lengths(unique)?;
    let ctx = StreamCtx::prop(StreamType::Length(LengthType::Dictionary), name);
    codecs.write_int_stream(&lengths, &ctx, enc)?;
    write_str_blob02(unique, enc)
}

/// [`StrLayout::Fsst`]: one length per value, then the symbol table and the compressed corpus.
#[cfg(feature = "unstable-v2")]
fn write_str_fsst02(
    raw: &FsstRawData,
    name: &str,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    write_str_leading02(&raw.value_lengths, StrLayout::Fsst, name, enc, codecs)?;
    write_fsst_tail02(
        &raw.symbol_lengths,
        &raw.symbol_bytes,
        &raw.corpus,
        BlobLayout::Plain,
        name,
        enc,
        codecs,
    )
}

/// [`StrLayout::FsstDict`]: one code per value, then the distinct values' lengths, symbol table and corpus.
#[cfg(feature = "unstable-v2")]
fn write_str_fsst_dict02(
    raw: &FsstRawData,
    offset_indices: &[u32],
    name: &str,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    write_str_leading02(offset_indices, StrLayout::FsstDict, name, enc, codecs)?;
    let ctx = StreamCtx::prop(StreamType::Length(LengthType::Dictionary), name);
    codecs.write_int_stream(&raw.value_lengths, &ctx, enc)?;
    write_fsst_tail02(
        &raw.symbol_lengths,
        &raw.symbol_bytes,
        &raw.corpus,
        BlobLayout::Plain,
        name,
        enc,
        codecs,
    )
}

/// Reorder a dictionary lexicographically, remapping the codes that index it.
///
/// Front coding only pays when neighbours share prefixes, which sorting is what arranges.
/// Equal values stay equal codes, so RLE over the codes is unaffected; their order is not.
#[cfg(feature = "unstable-v2")]
fn sort_dictionary<'a>(unique: &[&'a str], codes: &[u32]) -> MltResult<(Vec<&'a str>, Vec<u32>)> {
    let mut order: Vec<u32> = (0..u32::try_from(unique.len())?).collect();
    order.sort_unstable_by_key(|&i| unique[i.into_usize()]);
    let mut rank = vec![0_u32; unique.len()];
    for (new, &old) in order.iter().enumerate() {
        rank[old.into_usize()] = u32::try_from(new)?;
    }
    let sorted = order.iter().map(|&i| unique[i.into_usize()]).collect();
    let recoded = codes.iter().map(|&c| rank[c.into_usize()]).collect();
    Ok((sorted, recoded))
}

/// The suffixes as one slice per entry, which is what FSST trains its symbols on.
#[cfg(feature = "unstable-v2")]
fn suffix_parts(coded: &FrontCoded) -> Vec<&[u8]> {
    let mut parts = Vec::with_capacity(coded.suffix_lengths.len());
    let mut at = 0_usize;
    for &len in &coded.suffix_lengths {
        let len = len.into_usize();
        parts.push(&coded.suffixes[at..at + len]);
        at += len;
    }
    parts
}

/// Write a front-coded dictionary's lengths stream, which holds the prefix then the suffix lengths.
#[cfg(feature = "unstable-v2")]
fn write_front_lengths02(
    coded: &FrontCoded,
    name: &str,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    let ctx = StreamCtx::prop(StreamType::Length(LengthType::Dictionary), name);
    codecs.write_int_stream(&coded.to_lengths(), &ctx, enc)
}

/// [`StrLayout::Dict`] over a front-coded dictionary: one code per value, then the shared-prefix
/// and suffix lengths, then the suffix bytes.
#[cfg(feature = "unstable-v2")]
fn write_str_front_dict02(
    coded: &FrontCoded,
    offset_indices: &[u32],
    name: &str,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    write_str_leading02(offset_indices, StrLayout::Dict, name, enc, codecs)?;
    write_front_lengths02(coded, name, enc, codecs)?;
    write_blob02(&coded.suffixes, BlobLayout::FrontCoded, enc)
}

/// [`StrLayout::FsstDict`] over a front-coded dictionary, so FSST runs over the suffixes.
#[cfg(feature = "unstable-v2")]
fn write_str_fsst_front_dict02(
    coded: &FrontCoded,
    blob: &FsstBlob,
    offset_indices: &[u32],
    name: &str,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    write_str_leading02(offset_indices, StrLayout::FsstDict, name, enc, codecs)?;
    write_front_lengths02(coded, name, enc, codecs)?;
    write_fsst_tail02(
        &blob.symbol_lengths,
        &blob.symbol_bytes,
        &blob.corpus,
        BlobLayout::FrontCoded,
        name,
        enc,
        codecs,
    )
}

/// The symbol lengths, symbol table and compressed corpus both v2 FSST layouts end with.
#[cfg(feature = "unstable-v2")]
fn write_fsst_tail02(
    symbol_lengths: &[u32],
    symbol_bytes: &[u8],
    corpus: &[u8],
    corpus_layout: BlobLayout,
    name: &str,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    let ctx = StreamCtx::prop(StreamType::Length(LengthType::Symbol), name);
    codecs.write_int_stream(symbol_lengths, &ctx, enc)?;
    write_blob02(symbol_bytes, BlobLayout::Plain, enc)?;
    write_blob02(corpus, corpus_layout, enc)
}

/// Encode with plain (`VarBinary` lengths) layout.
///
/// Stream count varint is written first, then presence, then the lengths stream
/// (via [`Codecs::write_int_stream`] which handles the explicit/auto dispatch internally),
/// then the raw string bytes as a plain unencoded data stream.
#[hotpath::measure]
fn write_str_plain(
    non_null: &[&str],
    presence: Option<&StagedStrings>,
    name: &str,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    let lengths = strings_to_lengths(non_null)?;
    enc.write_varint(2u32 + u32::from(presence.is_some()))?;
    write_presence_stream(presence, enc, codecs)?;
    let ctx = StreamCtx::prop(StreamType::Length(LengthType::VarBinary), name);
    codecs.write_int_stream(&lengths, &ctx, enc)?;
    write_raw_str_data(non_null, DictionaryType::None, enc)
}

/// Encode with dictionary (deduped corpus + offset indices) layout.
#[hotpath::measure]
fn write_str_dict(
    non_null: &[&str],
    presence: Option<&StagedStrings>,
    name: &str,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    let (unique, offset_indices) = dedup_strings(non_null)?;
    write_str_dict_raw(&unique, &offset_indices, presence, name, enc, codecs)
}

/// Write pre-deduped dictionary data.
fn write_str_dict_raw(
    unique: &[&str],
    offset_indices: &[u32],
    presence: Option<&StagedStrings>,
    name: &str,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    let lengths = strings_to_lengths(unique)?;
    enc.write_varint(3u32 + u32::from(presence.is_some()))?;
    write_presence_stream(presence, enc, codecs)?;

    let ctx = StreamCtx::prop(StreamType::Length(LengthType::Dictionary), name);
    codecs.write_int_stream(&lengths, &ctx, enc)?;

    let ctx = StreamCtx::prop(StreamType::Offset(OffsetType::String), name);
    codecs.write_int_stream(offset_indices, &ctx, enc)?;
    write_raw_str_data(unique, DictionaryType::Single, enc)
}

/// Encode with FSST compression, training a fresh compressor.
///
/// Used by the explicit-encoder path.
#[hotpath::measure]
fn write_str_fsst(
    non_null: &[&str],
    presence: Option<&StagedStrings>,
    name: &str,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    let raw = compress_fsst(non_null);
    write_str_fsst_raw(&raw, non_null.len(), presence, name, enc, codecs)
}

/// Shared FSST write logic.
fn write_str_fsst_raw(
    raw: &FsstRawData,
    count: usize,
    presence: Option<&StagedStrings>,
    name: &str,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    let offsets: Vec<u32> = (0..u32::try_from(count)?).collect();
    enc.write_varint(5u32 + u32::from(presence.is_some()))?;
    write_presence_stream(presence, enc, codecs)?;
    write_fsst_data(raw, DictionaryType::Single, name, enc, codecs)?;
    let ctx = StreamCtx::prop(StreamType::Offset(OffsetType::String), name);
    codecs.write_int_stream(&offsets, &ctx, enc)
}

/// Encode with FSST + dictionary layout, training a fresh compressor.
///
/// Used by the explicit-encoder path.
#[hotpath::measure]
fn write_str_fsst_dict(
    non_null: &[&str],
    presence: Option<&StagedStrings>,
    name: &str,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    let (unique, offset_indices) = dedup_strings(non_null)?;
    let raw = compress_fsst(&unique);
    write_str_fsst_dict_raw(&raw, &offset_indices, presence, name, enc, codecs)
}

/// Shared FSST+dict write logic.
fn write_str_fsst_dict_raw(
    raw: &FsstRawData,
    offset_indices: &[u32],
    presence: Option<&StagedStrings>,
    name: &str,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    enc.write_varint(5u32 + u32::from(presence.is_some()))?;
    write_presence_stream(presence, enc, codecs)?;
    write_fsst_data(raw, DictionaryType::Single, name, enc, codecs)?;
    let ctx = StreamCtx::prop(StreamType::Offset(OffsetType::String), name);
    codecs.write_int_stream(offset_indices, &ctx, enc)
}

fn write_presence_stream(
    presence: Option<&StagedStrings>,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    if let Some(strings) = presence {
        codecs.write_presence_stream(strings.presence_bools(), enc)?;
    }
    Ok(())
}

/// Write 4 FSST sub-streams directly to `enc.data`.
///
/// The two integer sub-streams (`symbol_lengths`, `value_lengths`) use [`Codecs::write_int_stream`]
/// so explicit encoder overrides are honored and all candidates are competed automatically.
/// The two raw-byte sub-streams (`symbol_table`, `corpus`) are written without integer encoding.
///
/// Stream order: `symbol_lengths`, `symbol_table`, `value_lengths`, `corpus`.
#[hotpath::measure]
pub fn write_fsst_data(
    raw: &FsstRawData,
    dict_type: DictionaryType,
    name: &str,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    let ctx = StreamCtx::prop(StreamType::Length(LengthType::Symbol), name);
    codecs.write_int_stream(&raw.symbol_lengths, &ctx, enc)?;
    let typ = StreamType::Data(DictionaryType::Fsst);
    let meta = StreamMeta::new_none(typ, ValueKind::Int, raw.symbol_lengths.len())?;
    write_stream_payload(enc, meta, false, &raw.symbol_bytes)?;
    let ctx = StreamCtx::prop(StreamType::Length(LengthType::Dictionary), name);
    codecs.write_int_stream(&raw.value_lengths, &ctx, enc)?;
    let meta = StreamMeta::new_none(
        StreamType::Data(dict_type),
        ValueKind::Int,
        raw.value_lengths.len(),
    )?;
    write_stream_payload(enc, meta, false, &raw.corpus)?;
    Ok(())
}

/// Write raw string bytes as an unencoded data stream directly to `enc.data`.
#[hotpath::measure]
pub fn write_raw_str_data(
    strings: &[&str],
    dict_type: DictionaryType,
    enc: &mut Encoder,
) -> MltResult<()> {
    let total_len: usize = strings.iter().map(|s| s.len()).sum();
    let typ = StreamType::Data(dict_type);
    let meta = StreamMeta::new_none(typ, ValueKind::Int, strings.len())?;
    header01::write_stream_meta(&meta, enc, false, u32::try_from(total_len)?)?;
    enc.data_mut().reserve(total_len);
    for s in strings {
        enc.data_mut().extend_from_slice(s.as_bytes());
    }
    Ok(())
}

impl StagedStrings {
    /// Stages a string column where every row has a value (no nulls).
    ///
    /// `name` is the column key (e.g. shared-dict suffix or top-level property name).
    ///
    /// `values` can be any iterator of string fragments, for example `["a", "b"]`,
    /// `vec!["x".into(), "y".into()]`, or `some_vec.iter().map(|s| s.as_str())`.
    #[must_use]
    pub fn from_strings(
        name: impl Into<String>,
        values: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Self {
        let name = name.into();
        let iter = values.into_iter();
        let (lower, _) = iter.size_hint();
        let mut lengths = Vec::with_capacity(lower);
        let mut data = String::new();
        let mut end = 0_i32;
        for value in iter {
            let value = value.as_ref();
            end = checked_string_end(end, value.len())
                .expect("staged string corpus exceeds supported i32 range");
            lengths.push(end);
            data.push_str(value);
        }
        Self {
            name,
            lengths,
            data,
        }
    }

    /// Stages a string column with optional values (nulls encoded in the length stream).
    ///
    /// `name` is the column key (e.g. shared-dict suffix or top-level property name).
    ///
    /// `values` can be any iterator of optional string fragments, for example
    /// `vec![Some("a"), None]` or a `Vec<Option<String>>`.
    #[must_use]
    pub fn from_optional(
        name: impl Into<String>,
        values: impl IntoIterator<Item = Option<impl AsRef<str>>>,
    ) -> Self {
        let name = name.into();
        let iter = values.into_iter();
        let (lower, _) = iter.size_hint();
        let mut lengths = Vec::with_capacity(lower);
        let mut data = String::new();
        let mut end = 0_i32;
        for value in iter {
            match value {
                Some(value) => {
                    let value = value.as_ref();
                    end = checked_string_end(end, value.len())
                        .expect("staged string corpus exceeds supported i32 range");
                    lengths.push(end);
                    data.push_str(value);
                }
                None => lengths.push(encode_null_end(end)),
            }
        }
        Self {
            name,
            lengths,
            data,
        }
    }

    #[must_use]
    pub fn feature_count(&self) -> usize {
        self.lengths.len()
    }

    pub fn presence_bools(&self) -> impl ExactSizeIterator<Item = bool> + '_ {
        self.lengths.iter().map(|&end| end >= 0)
    }

    #[must_use]
    pub fn dense_values(&self) -> Vec<&str> {
        let mut values = Vec::new();
        let mut start = 0_u32;
        for &end in &self.lengths {
            if end >= 0 {
                let end = end.cast_unsigned();
                values.push(&self.data[start.into_usize()..end.into_usize()]);
                start = end;
            } else {
                start = (!end).cast_unsigned();
            }
        }
        values
    }
}
