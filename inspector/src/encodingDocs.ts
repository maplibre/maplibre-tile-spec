/** Docs sections, and which of them explain a region the dump hands over. */

import type { BlobInfo, DumpTree, Region } from "./annotate.ts";

export interface EncodingDoc {
  title: string;
  html: string;
  /** The section on the docs site, relative to the app's place inside it. */
  site: string;
  /** Where to edit this section, on the repo that publishes it. */
  edit: string;
}

/** A section, with the key it was asked for by. */
export interface Section extends EncodingDoc {
  key: string;
}

/**
 * Stream encodings the dump names, mapped to the section defining each.
 *
 * The keys are the ids `mlt ls` and the annotated dump both report: a logical id
 * carries its family, a physical id does not, which is what tells the two `none`s
 * apart. The wire side knows nothing about these pages, so this is where the two
 * vocabularies meet.
 */
const BY_ID: Record<string, string> = {
  // Logical, per family.
  "int/none": "encodings#logical-none",
  "str/none": "encodings#logical-none",
  "vertex/none": "encodings#logical-none",
  "bytes/none": "encodings#plain-bytes",
  "bool/none": "encodings#bitmap",
  "float/none": "encodings#plain-floats",
  "int/delta": "encodings#delta",
  "str/delta": "encodings#delta",
  "vertex/delta": "encodings#delta",
  "int/rle": "encodings#rle",
  "str/rle": "encodings#rle",
  "bool/rle": "encodings#boolean-rle",
  "bool/byte-rle": "encodings#boolean-rle",
  "float/rle": "encodings#boolean-rle",
  "int/delta-rle": "encodings#delta-rle",
  "str/delta-rle": "encodings#delta-rle",
  "int/bit-packed": "encodings#bit-packing",
  "str/bit-packed": "encodings#bit-packing",
  "vertex/componentwise-delta": "encodings#componentwise-delta",
  "vertex/morton": "encodings#morton",
  "vertex/morton-delta": "encodings#morton",
  "vertex/morton-rle": "encodings#morton",
  "float/alp": "encodings#alp",
  "float/dict": "encodings#float-dictionary",
  "bytes/front-coded": "encodings#front-coding",
  // Physical, which carries no family.
  none: "encodings#physical-none",
  varint: "encodings#varint",
  fastpfor: "encodings#fastpfor",
  "bit-packed": "encodings#bit-packing",
  // A string column's layout, from the extension bits.
  "layout/plain": "encodings#string-layouts",
  "layout/dict": "encodings#string-layouts",
  "layout/fsst": "encodings#fsst",
  "layout/fsst-dict": "encodings#fsst",
};

/** A section that reads the same whichever tag named the bytes. */
const same = (key: string) => ({ v1: key, v2: key });

/**
 * What a region is, by the label the dump gives it, for the parts of a tile that are
 * not a stream encoding. A label of the form `name[3]` is matched without its index.
 *
 * Most of these are tag-specific: the two specs lay the same idea out differently and
 * name it differently, so each tag points at its own page. A label with no entry for
 * the tag in hand simply shows no section, which is why v1 is thinner here.
 */
const BY_LABEL: Record<string, Partial<Record<"v1" | "v2", string>>> = {
  // The framing every layer starts with, which names no stream of its own.
  layer: { v1: "v1#column-metadata", v2: "v2#tile-layout" },
  size: { v1: "v1#column-metadata", v2: "v2#tile-layout" },
  tag: { v1: "v1#column-metadata", v2: "v2#tile-layout" },
  extent: { v1: "v1#column-metadata", v2: "v2#extent" },
  column: { v1: "v1#column-data", v2: "v2#columns" },
  column_schema: { v1: "v1#column-metadata" },
  // v1 puts a stream's role and count on the wire, so its header has parts v2 has not.
  stream: { v1: "v1#stream-types" },
  stream_count: { v1: "v1#stream-types" },
  stream_type: { v1: "v1#stream-types" },
  meta: { v1: "v1#binary-structure" },
  dict_stream: { v1: "v1#string-columns" },
  id: { v1: "v1#id-column" },
  runs: same("encodings#rle"),
  num_rle_values: same("encodings#rle"),
  bits: same("encodings#morton"),
  shift: same("encodings#morton"),
  // Nested nodes, whose children the walker labels by their kind.
  field: { v2: "v2#struct-nodes" },
  field_count: { v2: "v2#counts" },
  element: { v2: "v2#list-nodes" },
  value: { v2: "v2#map-nodes" },
  values: { v2: "v2#leaf-nodes" },
  dictionary: same("encodings#shared-dictionary"),
  header: { v2: "v2#layer-header-byte" },
  layout: { v2: "v2#layer-layout-byte" },
  name: { v1: "v1#column-metadata", v2: "v2#column-names" },
  type: { v1: "v1#column-types", v2: "v2#column-type-byte" },
  encoding: { v1: "v1#encoding-byte", v2: "v2#encoding-byte" },
  num_values: { v1: "v1#encoding-byte", v2: "v2#value-count" },
  byte_length: { v1: "v1#encoding-byte", v2: "v2#byte-length" },
  present: { v1: "v1#column-metadata", v2: "v2#presence-nibble" },
  shared_presence: { v2: "v2#shared-presence-fields" },
  coding: { v2: "v2#presence-encodings" },
  feature_count: { v1: "v1#column-data", v2: "v2#layer-body" },
  column_count: { v1: "v1#column-metadata", v2: "v2#column-counts" },
  column_counts: { v2: "v2#column-counts" },
  types: { v1: "v1#geometry-types", v2: "v2#geometry-types" },
  geometry: { v1: "v1#binary-structure", v2: "v2#geometry-section" },
  geo_lengths: { v1: "v1#topology-encoding", v2: "v2#geometry-layout" },
  part_lengths: { v1: "v1#topology-encoding", v2: "v2#geometry-layout" },
  ring_lengths: { v1: "v1#topology-encoding", v2: "v2#geometry-layout" },
  lengths: { v1: "v1#string-columns", v2: "v2#string-columns" },
  dict_lengths: { v1: "v1#string-columns", v2: "v2#string-columns" },
  dict_values: { v1: "v1#string-columns", v2: "v2#string-columns" },
  codes: { v1: "v1#string-columns", v2: "v2#string-columns" },
  shared_dict: {
    v1: "v1#shared-dictionary-columns",
    v2: "v2#shared-dictionary-columns",
  },
  // The vertex streams, and the two codecs that split their parameters into fields of
  // their own: without these the header bytes beside a payload explain nothing.
  vertices: { v2: "v2#the-vertex-sequence" },
  vertex_dict: same("encodings#vertex-dictionary"),
  vertex_offsets: same("encodings#vertex-dictionary"),
  morton_bits: same("encodings#morton"),
  morton_shift: same("encodings#morton"),
  alp_base: same("encodings#alp"),
  alp_scale: same("encodings#alp"),
  z_step: { v2: "v2#z-coordinates" },
  shapes: { v2: "v2#nested-properties" },
  shape_ids: { v2: "v2#nested-properties" },
  shape_table: { v2: "v2#nested-properties" },
  keys: { v2: "v2#map-nodes" },
  "m-values": { v2: "v2#m-values" },
  m_value: { v2: "v2#m-values" },
  // v1 nests only a shared dictionary's columns; v2 nests struct, list and map nodes.
  child: {
    v1: "v1#shared-dictionary-columns",
    v2: "v2#where-nested-columns-may-appear",
  },
  child_count: { v1: "v1#shared-dictionary-columns", v2: "v2#counts" },
  children: { v2: "v2#counts" },
  // The payload of these is the codec's, the same under either tag.
  symbol_lengths: same("encodings#fsst"),
  symbol_table: same("encodings#fsst"),
  corpus: same("encodings#fsst"),
  // Not shared: v2 stores only an all-polygon layer's triangles and counts indices
  // from the layer's first vertex, so v1's section would describe the wrong bytes.
  tri_lengths: { v1: "v1#tessellation-data-optional", v2: "v2#tessellation" },
  tri_indexes: { v1: "v1#tessellation-data-optional", v2: "v2#tessellation" },
};

/** A codec's parameters in a stream header, whose bytes the payload's encodings do not pack. */
const CODEC_PARAMETERS = new Set([
  "morton_bits",
  "morton_shift",
  "alp_base",
  "alp_scale",
  "z_step",
]);

/** The spec page describing the tile in hand, since the two tags lay bytes out differently. */
export function specPage(tree: DumpTree | null): "v1" | "v2" {
  const tag = tree?.regions.find((region) => region.label === "tag")?.value;
  return tag?.includes("Tag01") === true ? "v1" : "v2";
}

let pending: Promise<Record<string, EncodingDoc>> | null = null;

/** Fetched once and kept: the sections are static, and most sessions open none of them. */
export function encodingDocs(): Promise<Record<string, EncodingDoc>> {
  pending ??= fetch(`${import.meta.env.BASE_URL}encodings.json`)
    .then((response) => (response.ok ? response.json() : {}))
    .catch(() => ({}));
  return pending;
}

/** Only for tests, which cannot share one module-level fetch across cases. */
export function resetEncodingDocs(): void {
  pending = null;
}

/** Every section key this app can ask for, so a test can check they all resolve. */
export function everyDocKey(): string[] {
  const labels = Object.values(BY_LABEL).flatMap((per) => Object.values(per));
  return [...new Set([...Object.values(BY_ID), ...labels])];
}

/** The named sections, deduplicated, in the order asked for. */
export function sectionsFor(
  keys: readonly string[],
  docs: Record<string, EncodingDoc>,
): Section[] {
  const seen = new Set<string>();
  const out: Section[] = [];
  for (const key of keys) {
    if (seen.has(key)) continue;
    const doc = docs[key];
    if (doc === undefined) continue;
    seen.add(key);
    out.push({ ...doc, key });
  }
  return out;
}

/**
 * Looks an encoding id up, ignoring the variant a codec spells in brackets.
 *
 * A stream reports `fastpfor[128le]` where the page documents FastPFOR once: the block
 * size and byte order pick a layout of the same codec, not another encoding.
 */
export function docKeyFor(id: string): string | undefined {
  return BY_ID[id] ?? BY_ID[id.replace(/\[.*\]$/, "")];
}

/**
 * The payload a stream holds directly, or null.
 *
 * Direct, not anywhere below: a stream's header fields and its `data` are siblings,
 * while a layer's streams are nested further down. That is what keeps `size` from
 * borrowing the encoding of a stream that merely happens to sit inside the layer.
 */
function ownBlob(regions: readonly Region[], index: number): BlobInfo | null {
  const region = regions[index];
  if (region === undefined) return null;
  const depth = region.depth;
  let only: BlobInfo | null = null;
  for (
    let i = index + 1;
    i < regions.length && regions[i].depth > depth;
    i += 1
  ) {
    if (regions[i].depth !== depth + 1) continue;
    const blob = regions[i].blob;
    if (blob === null) continue;
    if (only !== null) return null;
    only = blob;
  }
  return only;
}

/**
 * The encodings the bytes at `index` are packed with, or none.
 *
 * A stream's header fields carry no encoding of their own, so `encoding` or
 * `num_values` reads the payload beside them: they are one stream, described once.
 */
function streamEncodings(regions: readonly Region[], index: number): string[] {
  const here = regions[index];
  if (here === undefined) return [];
  const own = here.blob ?? ownBlob(regions, index);
  if (own !== null) return [own.logical, own.physical];
  let at = index - 1;
  while (at >= 0 && regions[at].depth >= here.depth) at -= 1;
  const beside = at < 0 ? null : ownBlob(regions, at);
  return beside === null ? [] : [beside.logical, beside.physical];
}

/**
 * The sections explaining the region at `index`: what it is, then the encodings it
 * names.
 *
 * Its own kind reads first, since a reader wants to know what a byte is before how
 * it is packed.
 */
export function regionAnchors(
  regions: readonly Region[],
  index: number | null,
  page: "v1" | "v2" = "v2",
): string[] {
  const region = index === null ? undefined : regions[index];
  if (region === undefined || index === null) return [];
  // `column[2]` and `m_value[0]` are the same kind of thing as `column` and `m_value`.
  const label = region.label.replace(/\[\d+\].*$/, "");
  const own = BY_LABEL[label]?.[page];
  const keys = own === undefined ? [] : [own];
  if (CODEC_PARAMETERS.has(label)) return keys;
  for (const id of streamEncodings(regions, index)) {
    const key = docKeyFor(id);
    if (key !== undefined) keys.push(key);
  }
  return keys;
}
