use mlt_core::dump::{DecodedBlob, DumpTree, annotate_tile, decode_blob, filter_layer};
use mlt_core::{Decoder, MltError};
use serde_wasm_bindgen::Serializer;
use wasm_bindgen::prelude::*;

/// A tile annotated as far as the walk got, plus the error that stopped it.
///
/// The handle keeps the tile bytes and the region list on the WASM side.
/// JS pulls the region list across once per layer and asks for decoded payloads one blob at a
/// time, so a tile with tens of thousands of regions costs one conversion rather than one per
/// value.
#[wasm_bindgen]
pub struct AnnotatedTile {
    buf: Vec<u8>,
    tree: DumpTree,
    error: Option<String>,
    dec: Decoder,
    /// Trees already handed to JS, keyed by the `layer` argument that produced them.
    cache: Vec<(Option<usize>, JsValue)>,
}

/// Annotate a raw MLT tile blob, region by region.
///
/// Never throws for a malformed tile: the walk hands back what it annotated and
/// [`AnnotatedTile::error`] carries the failure.
#[wasm_bindgen(js_name = "annotateTile")]
#[must_use]
pub fn annotate(data: &[u8]) -> AnnotatedTile {
    let (tree, err) = annotate_tile(data);
    AnnotatedTile {
        buf: data.to_vec(),
        tree,
        error: err.as_ref().map(MltError::to_string),
        dec: Decoder::default(),
        cache: Vec::new(),
    }
}

#[wasm_bindgen]
impl AnnotatedTile {
    /// The regions of the whole tile, or of one top-level layer, memoized per argument.
    ///
    /// A tree costs a real conversion, so this is a method rather than a getter.
    pub fn tree(&mut self, layer: Option<usize>) -> Result<JsValue, JsError> {
        if let Some((_, cached)) = self.cache.iter().find(|(key, _)| *key == layer) {
            return Ok(cached.clone());
        }
        let value = match layer {
            None => to_js(&self.tree)?,
            Some(idx) => {
                let filtered = filter_layer(&self.tree, idx)
                    .ok_or_else(|| JsError::new(&format!("the tile has no layer {idx}")))?;
                to_js(&filtered)?
            }
        };
        self.cache.push((layer, value.clone()));
        Ok(value)
    }

    /// The walker failure that stopped the annotation, or `null`.
    ///
    /// When set, the tree ends in an `<unannotated>` leaf covering the bytes the walk never
    /// reached.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn error(&self) -> JsValue {
        self.error
            .as_deref()
            .map_or(JsValue::NULL, JsValue::from_str)
    }

    /// Decode the payload of the data blob at `region_index`, keeping at most `max_values` values.
    ///
    /// `max_values` of `0` keeps all of them.
    #[wasm_bindgen(js_name = "decodeBlob")]
    pub fn decode_blob(
        &mut self,
        region_index: usize,
        max_values: usize,
    ) -> Result<JsValue, JsError> {
        let region = self
            .tree
            .regions
            .get(region_index)
            .ok_or_else(|| JsError::new(&format!("the tree has no region {region_index}")))?;
        let bytes = self.buf.get(region.offset..region.offset + region.len);
        let decoded = match (region.blob, bytes) {
            (Some(info), Some(bytes)) => decode_blob(info, bytes, max_values, &mut self.dec),
            (None, _) => DecodedBlob::Error {
                message: format!("region {region_index} carries no stream metadata"),
            },
            (_, None) => DecodedBlob::Error {
                message: format!("region {region_index} lies outside the tile buffer"),
            },
        };
        to_js(&decoded)
    }
}

/// Offsets and counts cross as numbers, 64-bit payload values as `BigInt`, and `None` as `null`.
fn to_js<T: serde::Serialize>(value: &T) -> Result<JsValue, JsError> {
    let ser = Serializer::new()
        .serialize_missing_as_null(true)
        .serialize_large_number_types_as_bigints(true);
    value.serialize(&ser).map_err(Into::into)
}
