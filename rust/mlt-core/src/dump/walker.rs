//! Shared scaffolding for the tag-specific annotating walkers.
//!
//! [`Walker`] records an annotated [`Region`] per field while the per-tag
//! `walker01` / `walker02` modules mirror their wire layout.
//! Tile and layer framing is shared - only the layer body differs per tag.

use super::UNANNOTATED;
use super::model::{BitField, BlobInfo, DumpTree, Region, RegionKind};
use crate::codecs::varint::parse_varint;
use crate::utils::{parse_u8, take};
use crate::{MltError, MltRefResult, MltResult, Parser};

/// Walk a whole tile buffer, producing an annotated [`DumpTree`].
///
/// The returned tree references offsets into `buf`; keep `buf` alive to render it.
/// A walk that bails still hands back everything it annotated, sealed into a tree whose
/// leaves partition the buffer, alongside the error that stopped it.
#[must_use]
pub fn annotate_tile(buf: &[u8]) -> (DumpTree, Option<MltError>) {
    let mut w = Walker {
        buf,
        out: Vec::new(),
        depth: 0,
        open_containers: Vec::new(),
        parser: Parser::default(),
    };
    let err = w.walk_tile().err();
    if err.is_some() {
        w.seal_partial();
    }
    let tree = DumpTree {
        buf_len: buf.len(),
        regions: w.out,
    };
    (tree, err)
}

pub(super) struct Walker<'a> {
    pub(super) buf: &'a [u8],
    pub(super) out: Vec<Region>,
    pub(super) depth: usize,
    /// Containers opened but not yet closed, innermost last.
    open_containers: Vec<usize>,
    /// Throwaway budget for the authoritative stream-header parsers.
    pub(super) parser: Parser,
}

impl<'a> Walker<'a> {
    /// Absolute offset of `s`, which must be a subslice of the tile buffer.
    pub(super) fn off(&self, s: &'a [u8]) -> usize {
        self.buf
            .subslice_range(s)
            .expect("annotated slice must come from the tile buffer")
            .start
    }

    /// Open a container region spanning children; returns its index for [`Walker::close`].
    pub(super) fn open(&mut self, at: &'a [u8], label: impl Into<String>) -> usize {
        let idx = self.out.len();
        self.out.push(Region {
            offset: self.off(at),
            len: 0,
            depth: self.depth,
            label: label.into(),
            value: None,
            bits: Vec::new(),
            kind: RegionKind::Meta,
            container: true,
            blob: None,
        });
        self.depth += 1;
        self.open_containers.push(idx);
        idx
    }

    /// Close the container opened at `idx`, ending it at `after`.
    pub(super) fn close(&mut self, idx: usize, after: &'a [u8]) {
        self.depth -= 1;
        // Popped outside the assert: `debug_assert_eq!` drops its arguments in release, which
        // would leave every closed container open for `seal_partial` to stretch.
        let closed = self.open_containers.pop();
        debug_assert_eq!(closed, Some(idx), "containers must close innermost first");
        let start = self.out[idx].offset;
        self.out[idx].len = self.off(after) - start;
    }

    /// Rename the container opened at `idx`.
    #[cfg(feature = "unstable-v2")]
    pub(super) fn relabel(&mut self, idx: usize, label: impl Into<String>) {
        self.out[idx].label = label.into();
    }

    pub(super) fn leaf(
        &mut self,
        before: &'a [u8],
        after: &'a [u8],
        label: impl Into<String>,
        value: Option<String>,
    ) {
        self.out.push(Region {
            offset: self.off(before),
            len: before.len() - after.len(),
            depth: self.depth,
            label: label.into(),
            value,
            bits: Vec::new(),
            kind: RegionKind::Meta,
            container: false,
            blob: None,
        });
    }

    /// Record a leaf metadata region carrying a bit-level breakdown.
    pub(super) fn leaf_bits(
        &mut self,
        before: &'a [u8],
        after: &'a [u8],
        label: impl Into<String>,
        value: Option<String>,
        bits: Vec<BitField>,
    ) {
        self.out.push(Region {
            offset: self.off(before),
            len: before.len() - after.len(),
            depth: self.depth,
            label: label.into(),
            value,
            bits,
            kind: RegionKind::Meta,
            container: false,
            blob: None,
        });
    }

    /// Parse one field with a real primitive, record a leaf region, return the tail.
    pub(super) fn field<T>(
        &mut self,
        before: &'a [u8],
        label: &str,
        parse: impl FnOnce(&'a [u8]) -> MltRefResult<'a, T>,
        render: impl FnOnce(&T) -> Option<String>,
    ) -> MltResult<(&'a [u8], T)> {
        let (after, val) = parse(before)?;
        let value = render(&val);
        self.leaf(before, after, label, value);
        Ok((after, val))
    }

    /// Parse one packed byte, record it with a bit-level breakdown, return the tail.
    pub(super) fn byte_field(
        &mut self,
        before: &'a [u8],
        label: &str,
        value: impl FnOnce(u8) -> String,
        bits: impl FnOnce(u8) -> Vec<BitField>,
    ) -> MltResult<(&'a [u8], u8)> {
        let (after, byte) = parse_u8(before)?;
        self.leaf_bits(before, after, label, Some(value(byte)), bits(byte));
        Ok((after, byte))
    }

    /// Record `len` bytes at `at` as a data blob (no decodable metadata).
    pub(super) fn raw_blob(&mut self, at: &'a [u8], len: usize, label: impl Into<String>) {
        self.blob(at, len, label.into(), None);
    }

    /// Record `len` bytes at `at` as a stream payload.
    pub(super) fn stream_blob(
        &mut self,
        at: &'a [u8],
        len: usize,
        label: impl Into<String>,
        info: BlobInfo,
    ) {
        self.blob(at, len, label.into(), Some(info));
    }

    fn blob(&mut self, at: &'a [u8], len: usize, label: String, blob: Option<BlobInfo>) {
        self.out.push(Region {
            offset: self.off(at),
            len,
            depth: self.depth,
            label,
            value: None,
            bits: Vec::new(),
            kind: RegionKind::DataBlob,
            container: false,
            blob,
        });
    }

    /// Turn the regions of a bailed-out walk into a valid tree.
    ///
    /// A container left open keeps `len == 0`, which hides everything inside it from a
    /// span query such as [`super::filter_layer`], and the bytes past the annotated
    /// prefix would otherwise leave the leaves not partitioning the buffer.
    fn seal_partial(&mut self) {
        let end = self
            .out
            .iter()
            .filter(|r| !r.container)
            .map(|r| r.offset + r.len)
            .max()
            .unwrap_or(0);

        for idx in self.open_containers.drain(..) {
            let offset = self.out[idx].offset;
            debug_assert!(
                offset <= end,
                "a container opens at the cursor, never past it"
            );
            self.out[idx].len = end.saturating_sub(offset);
        }

        if end < self.buf.len() {
            self.out.push(Region {
                offset: end,
                len: self.buf.len() - end,
                depth: 0,
                label: UNANNOTATED.to_string(),
                value: None,
                bits: Vec::new(),
                kind: RegionKind::DataBlob,
                container: false,
                blob: None,
            });
        }
    }

    fn walk_tile(&mut self) -> MltResult<()> {
        let mut input = self.buf;
        let mut idx = 0;
        while !input.is_empty() {
            input = self.walk_layer(input, idx)?;
            idx += 1;
        }
        Ok(())
    }

    /// Mirror [`crate::decoder::Layer`]`::from_bytes`: `[varint size][u8 tag][value]`.
    fn walk_layer(&mut self, input: &'a [u8], idx: usize) -> MltResult<&'a [u8]> {
        let ci = self.open(input, format!("layer[{idx}]"));

        let (input, size) = self.field(
            input,
            "size",
            |i| parse_varint::<u32>(i),
            |v| Some(format!("{v} (varint) - tag + body")),
        )?;
        let (input, tag) = self.field(input, "tag", parse_u8, |t| Some(tag_label(*t)))?;

        let body_len = size.checked_sub(1).ok_or(MltError::ZeroLayerSize)?;
        let (rest, body) = take(input, body_len)?;

        match tag {
            1 => self.walk_layer01(body)?,
            #[cfg(feature = "unstable-v2")]
            2 => self.walk_layer02(body)?,
            _ => self.raw_blob(body, body.len(), format!("value (Unknown tag 0x{tag:02X})")),
        }

        self.close(ci, rest);
        Ok(rest)
    }
}

/// How the parser will read this layer tag.
fn tag_label(tag: u8) -> String {
    match tag {
        1 => "0x01 -> Tag01".into(),
        #[cfg(feature = "unstable-v2")]
        2 => "0x02 -> Tag02".into(),
        other => format!("0x{other:02X} -> Unknown"),
    }
}
