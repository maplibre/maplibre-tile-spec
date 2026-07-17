//! Renders a [`DumpTree`] as an annotated hexdump.

use std::io::{self, Write};

use super::model::{BlobInfo, DecodeHint, DumpTree, Region, RegionKind};
use crate::Decoder;
use crate::decoder::RawStream;

/// How data-payload blobs are shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DataMode {
    /// Raw hex plus best-effort decoded values (default).
    #[default]
    Both,
    /// Raw hex only.
    Blob,
    /// Decoded values only.
    Decoded,
    /// A one-line summary, no payload bytes.
    Hidden,
}

/// Rendering options for [`render`].
#[derive(Debug, Clone, Copy)]
pub struct RenderOpts {
    /// Hex bytes per row.
    pub width: usize,
    /// Show the bit-level breakdown of packed bytes.
    pub show_bits: bool,
    /// Emit ANSI color escapes.
    pub color: bool,
    /// How to render data payloads.
    pub data_mode: DataMode,
    /// Truncate raw payload hex to this many bytes (`0` = unlimited).
    pub max_blob: usize,
}

impl Default for RenderOpts {
    fn default() -> Self {
        Self {
            width: 16,
            show_bits: true,
            color: false,
            data_mode: DataMode::Both,
            max_blob: 256,
        }
    }
}

const SEP: &str = " | ";

/// Render `tree` as an annotated hexdump. `buf` must be the same buffer that was
/// passed to [`super::annotate_tile`].
pub fn render(
    tree: &DumpTree,
    buf: &[u8],
    opts: &RenderOpts,
    w: &mut impl Write,
) -> io::Result<()> {
    let mut dec = Decoder::default();
    let left_len = left_width(opts.width);
    for region in &tree.regions {
        render_region(w, buf, region, opts, left_len, &mut dec)?;
    }
    Ok(())
}

fn left_width(width: usize) -> usize {
    // "{off:08x}  " + hex(width*3) + "  " + ascii(width)
    8 + 2 + width * 3 + 2 + width
}

fn render_region(
    w: &mut impl Write,
    buf: &[u8],
    region: &Region,
    opts: &RenderOpts,
    left_len: usize,
    dec: &mut Decoder,
) -> io::Result<()> {
    let indent = "  ".repeat(region.depth);

    if region.container {
        let left = format!("{:08x}", region.offset);
        let annot = format!("{indent}{} ({} B)", paint(&region.label, opts, Paint::Label), region.len);
        writeln!(w, "{left:<left_len$}{SEP}{annot}")?;
        return Ok(());
    }

    let bytes = &buf[region.offset..region.offset + region.len];

    match region.kind {
        RegionKind::Meta => render_meta(w, region, bytes, opts, left_len, &indent),
        RegionKind::DataBlob => render_blob(w, region, bytes, opts, left_len, &indent, dec),
    }
}

fn render_meta(
    w: &mut impl Write,
    region: &Region,
    bytes: &[u8],
    opts: &RenderOpts,
    left_len: usize,
    indent: &str,
) -> io::Result<()> {
    let annot = match &region.value {
        Some(v) => format!(
            "{indent}{}: {}",
            paint(&region.label, opts, Paint::Label),
            paint(v, opts, Paint::Value)
        ),
        None => format!("{indent}{}", paint(&region.label, opts, Paint::Label)),
    };
    emit_bytes(w, region.offset, bytes, opts, left_len, &annot)?;

    if opts.show_bits {
        for bf in &region.bits {
            let range = if bf.hi == bf.lo {
                format!("bit {}", bf.hi)
            } else {
                format!("bits {}-{}", bf.hi, bf.lo)
            };
            let width = usize::from(bf.hi - bf.lo + 1);
            let annot = format!(
                "{indent}  └ {range} = {:0width$b} → {}",
                bf.raw,
                bf.meaning,
                width = width
            );
            writeln!(w, "{:<left_len$}{SEP}{}", "", paint(&annot, opts, Paint::Dim))?;
        }
    }
    Ok(())
}

fn render_blob(
    w: &mut impl Write,
    region: &Region,
    bytes: &[u8],
    opts: &RenderOpts,
    left_len: usize,
    indent: &str,
    dec: &mut Decoder,
) -> io::Result<()> {
    let summary = blob_summary(region, bytes);

    if opts.data_mode == DataMode::Hidden {
        let annot = format!("{indent}{}", paint(&summary, opts, Paint::Dim));
        let left = format!("{:08x}", region.offset);
        writeln!(w, "{left:<left_len$}{SEP}{annot}")?;
        return Ok(());
    }

    // Raw hex (unless Decoded-only).
    if matches!(opts.data_mode, DataMode::Both | DataMode::Blob) {
        let annot = format!("{indent}{}", paint(&summary, opts, Paint::Dim));
        let shown = if opts.max_blob == 0 || bytes.len() <= opts.max_blob {
            bytes
        } else {
            &bytes[..opts.max_blob]
        };
        emit_bytes(w, region.offset, shown, opts, left_len, &annot)?;
        if shown.len() < bytes.len() {
            let note = format!(
                "{indent}  … {} more bytes omitted (--max-blob to change)",
                bytes.len() - shown.len()
            );
            writeln!(w, "{:<left_len$}{SEP}{}", "", paint(&note, opts, Paint::Dim))?;
        }
    }

    // Decoded values (unless Blob-only).
    if matches!(opts.data_mode, DataMode::Both | DataMode::Decoded)
        && let Some(info) = region.blob
    {
        let decoded = decode_blob(info, bytes, dec);
        let annot = format!("{indent}  decoded: {}", paint(&decoded, opts, Paint::Value));
        writeln!(w, "{:<left_len$}{SEP}{}", "", annot)?;
    }
    Ok(())
}

/// One-line description of a data blob for its annotation column.
fn blob_summary(region: &Region, bytes: &[u8]) -> String {
    match region.blob {
        Some(info) => format!(
            "{} [{:?} {:?}/{:?}, {} values, {} B]",
            region.label,
            info.meta.stream_type,
            info.meta.encoding.logical,
            info.meta.encoding.physical,
            info.meta.num_values,
            bytes.len()
        ),
        None => format!("{} [{} B]", region.label, bytes.len()),
    }
}

/// Best-effort decode of a stream payload for display. Never panics — decode
/// errors are rendered inline.
fn decode_blob(info: BlobInfo, data: &[u8], dec: &mut Decoder) -> String {
    // Bound memory/time per blob: the decoded values are dropped immediately.
    dec.reset_budget();
    let meta = info.meta;
    match info.hint {
        DecodeHint::Presence => match RawStream::new(meta, data).decode_bitvec(dec) {
            Ok(bits) => {
                let n = bits.len();
                let shown: String = bits.iter().take(96).map(|b| if *b { '1' } else { '0' }).collect();
                let more = if n > 96 { "…" } else { "" };
                format!("{n} present-bits: {shown}{more}")
            }
            Err(e) => format!("<undecodable: {e}>"),
        },
        DecodeHint::Bool => fmt_res(RawStream::new(meta, data).decode_bools(dec)),
        DecodeHint::I32 => fmt_res(RawStream::new(meta, data).decode_ints::<i32>(dec)),
        DecodeHint::U32 => fmt_res(RawStream::new(meta, data).decode_ints::<u32>(dec)),
        DecodeHint::I64 => fmt_res(RawStream::new(meta, data).decode_ints::<i64>(dec)),
        DecodeHint::U64 => fmt_res(RawStream::new(meta, data).decode_ints::<u64>(dec)),
        DecodeHint::F32 => fmt_res(RawStream::new(meta, data).decode_floats::<f32>(dec)),
        DecodeHint::F64 => fmt_res(RawStream::new(meta, data).decode_floats::<f64>(dec)),
        DecodeHint::Bytes => match std::str::from_utf8(data) {
            Ok(s) => format!("utf-8 {:?}", truncate_str(s, 200)),
            Err(_) => format!("<{} binary bytes>", data.len()),
        },
    }
}

fn fmt_res<T: std::fmt::Display>(res: crate::MltResult<Vec<T>>) -> String {
    match res {
        Ok(v) => fmt_list(&v),
        Err(e) => format!("<undecodable: {e}>"),
    }
}

fn fmt_list<T: std::fmt::Display>(v: &[T]) -> String {
    const MAX: usize = 48;
    let shown = v
        .iter()
        .take(MAX)
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    if v.len() > MAX {
        format!("[{shown}, … ] ({} total)", v.len())
    } else {
        format!("[{shown}]")
    }
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let head: String = s.chars().take(max).collect();
        format!("{head}…")
    }
}

/// Emit one or more hexdump rows for `bytes` starting at `offset`. The
/// `annotation` is printed on the first row only.
fn emit_bytes(
    w: &mut impl Write,
    offset: usize,
    bytes: &[u8],
    opts: &RenderOpts,
    left_len: usize,
    annotation: &str,
) -> io::Result<()> {
    if bytes.is_empty() {
        let left = format!("{offset:08x}");
        writeln!(w, "{left:<left_len$}{SEP}{annotation}")?;
        return Ok(());
    }
    for (row, chunk) in bytes.chunks(opts.width).enumerate() {
        let row_off = offset + row * opts.width;
        let hex = chunk
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(" ");
        let ascii: String = chunk
            .iter()
            .map(|&b| if (0x20..=0x7e).contains(&b) { b as char } else { '.' })
            .collect();
        let hexw = opts.width * 3;
        let left = format!("{row_off:08x}  {hex:<hexw$}  {ascii}");
        if row == 0 {
            writeln!(w, "{left:<left_len$}{SEP}{annotation}")?;
        } else {
            writeln!(w, "{left:<left_len$}{SEP}")?;
        }
    }
    Ok(())
}

// ── Minimal ANSI coloring ────────────────────────────────────────────────────

#[derive(Clone, Copy)]
enum Paint {
    Label,
    Value,
    Dim,
}

fn paint(s: &str, opts: &RenderOpts, kind: Paint) -> String {
    if !opts.color {
        return s.to_string();
    }
    let code = match kind {
        Paint::Label => "1",    // bold
        Paint::Value => "36",   // cyan
        Paint::Dim => "2",      // dim
    };
    format!("\x1b[{code}m{s}\x1b[0m")
}
