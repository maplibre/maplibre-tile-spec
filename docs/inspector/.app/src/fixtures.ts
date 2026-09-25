/** The fixture index the app is built against, and the tiles it hands over. */

/** One fixture tile, as the build-time index records it. */
export interface FixtureEntry {
  /** File name including the `.mlt` extension. */
  name: string;
  /** Directory under `test/synthetic` the fixture lives in. */
  directory: string;
  /** Size of the tile on disk. */
  bytes: number;
  /**
   * Every axis `mlt ls --format json` reports, keyed as its `facets` object spells them:
   * extent, geometry, geomLayout, dataType, strLayout, dictLayout, streamType,
   * physical, logical. The values are the spec's own names, so they are shown as they are.
   *
   * Absent for a tile `mlt ls` could not read, which then carries no axis at all.
   */
  facets?: Record<string, string[]>;
}

/** Wire tag of the fixture's directory, e.g. `0x02` for `0x02-rust`. */
export function fixtureTag(entry: FixtureEntry): string {
  return entry.directory.split("-")[0];
}

/**
 * The v2 vocabulary, spelled as the spec names it.
 *
 * Written out rather than gathered from the index on purpose: a value no fixture
 * carries is the interesting one, and an index-derived list is exactly the list that
 * cannot show it. Every axis therefore offers what v2 *can* hold, and the counts say
 * what this index actually has.
 */

/** Section headings, in the order the sheet stacks them. */
export const SECTIONS = ["Geometry", "Columns", "Streams"] as const;
export type Section = (typeof SECTIONS)[number];

/**
 * Data types in reading order: booleans, signed, unsigned, floats, the string type, then
 * the tile flags. Alphabetical would file `i8` after `i64` and split each family up.
 */
const DATA_TYPES = [
  "bool",
  "i8",
  "i32",
  "i64",
  "u8",
  "u32",
  "u64",
  "f32",
  "f64",
  "str",
  "m-values",
];

/** One filter axis: where its values come from, and every value v2 defines for it. */
export interface Axis {
  /** Key the picked set and the deep link use, and the `facets` field it reads. */
  key: string;
  /** Row label in the sheet. */
  label: string;
  /** Section the row sits under, or `null` to sit above them all, always in view. */
  section: Section | null;
  /** What v2 can hold, in spec order. The index may carry none of it. */
  vocabulary: readonly string[];
  /** The values one entry carries. */
  of: (entry: FixtureEntry) => string[];
  /** Orders the values the vocabulary does not name. Alphabetical unless given. */
  sort?: (a: string, b: string) => number;
}

/** Reads one axis off the entry, tolerating a tile `mlt ls` could not read. */
const field =
  (key: string) =>
  (entry: FixtureEntry): string[] =>
    entry.facets?.[key] ?? [];

export const AXES: Axis[] = [
  {
    key: "tag",
    label: "tag",
    // Above the sections: which wire version a tile is written in is the one thing worth
    // narrowing by before anything else, and there are only two of them.
    section: null,
    vocabulary: ["0x01", "0x02"],
    of: (entry) => [fixtureTag(entry)],
  },
  {
    key: "geometry",
    label: "geometry",
    section: "Geometry",
    vocabulary: [
      "point",
      "line-string",
      "polygon",
      "multi-point",
      "multi-line-string",
      "multi-polygon",
    ],
    of: field("geometry"),
  },
  {
    key: "geomLayout",
    label: "layout",
    section: "Geometry",
    vocabulary: [
      "points",
      "points-dict",
      "multi-points",
      "multi-points-dict",
      "lines",
      "lines-dict",
      "multi-lines",
      "multi-lines-dict",
      "polygons",
      "polygons-dict",
      "multi-polygons",
      "multi-polygons-dict",
      "tess-polygons",
      "tess-polygons-with-outlines",
    ],
    of: field("geomLayout"),
  },
  {
    key: "extent",
    label: "extent",
    section: "Geometry",
    // Listed from the index rather than from the spec, unlike every other axis. v2 codes
    // a power of two from 64 to 2097152 and v1 restricts nothing, so no one vocabulary
    // covers the row: the corpus has both 32 and 1073741824. An extent is a measurement
    // of the tile rather than a feature to show off, so a value nothing uses is not a gap.
    vocabulary: [],
    sort: (a, b) => Number(a) - Number(b),
    of: field("extent"),
  },
  {
    key: "dataType",
    label: "types",
    section: "Columns",
    vocabulary: DATA_TYPES,
    of: field("dataType"),
  },
  {
    key: "strLayout",
    label: "strings",
    section: "Columns",
    vocabulary: ["plain", "dict", "fsst", "fsst-dict"],
    of: field("strLayout"),
  },
  {
    key: "dictLayout",
    label: "dictionary",
    section: "Columns",
    vocabulary: ["plain", "front-coded"],
    of: field("dictLayout"),
  },
  {
    key: "streamType",
    label: "stream",
    section: "Streams",
    vocabulary: [
      "present",
      "data",
      "data[single]",
      "data[shared]",
      "data[vertex]",
      "data[morton]",
      "data[fsst]",
      "offset[vertex]",
      "offset[index]",
      "offset[string]",
      "offset[key]",
      "length[varbinary]",
      "length[geometries]",
      "length[parts]",
      "length[rings]",
      "length[triangles]",
      "length[symbol]",
      "length[dictionary]",
      "length[nested]",
    ],
    of: field("streamType"),
  },
  {
    key: "physical",
    label: "physical",
    section: "Streams",
    vocabulary: ["varint", "bitpacked", "fastpfor[256be]", "fastpfor[128le]"],
    of: field("physical"),
  },
  {
    key: "logical",
    label: "logical",
    section: "Streams",
    vocabulary: [
      "delta",
      "rle",
      "delta-rle",
      "componentwise-delta",
      "morton",
      "morton-delta",
      "morton-rle",
      "dict",
      "alp",
    ],
    of: field("logical"),
  },
];

/** One chip: a value, what it would narrow to, and what the whole index holds. */
export interface AxisValue {
  value: string;
  /** Entries among the current matches that carry it: what picking it narrows to. */
  count: number;
  /** Entries in the whole index. `0` means no fixture shows this off at all. */
  total: number;
}

/** One row of chips. */
export interface AxisRow extends Axis {
  values: AxisValue[];
}

/** How many entries carry each value of `axis`. */
function tally(entries: FixtureEntry[], axis: Axis): Map<string, number> {
  const counts = new Map<string, number>();
  for (const entry of entries)
    for (const value of axis.of(entry))
      counts.set(value, (counts.get(value) ?? 0) + 1);
  return counts;
}

/**
 * Every axis, with its whole vocabulary counted twice: over `matching` for what a chip
 * would narrow to, and over `index` for whether any fixture has it at all.
 *
 * A value the vocabulary does not name is one this app has not met - a newer encoder, or
 * a spelling that drifted - so it is kept, after the named ones, rather than hidden.
 */
export function axisRows(
  index: FixtureEntry[],
  matching: FixtureEntry[],
): AxisRow[] {
  return AXES.map((axis) => {
    const here = tally(matching, axis);
    const all = tally(index, axis);
    const extra = [...all.keys()]
      .filter((value) => !axis.vocabulary.includes(value))
      .sort(axis.sort ?? compare);
    const values = [...axis.vocabulary, ...extra].map((value) => ({
      value,
      count: here.get(value) ?? 0,
      total: all.get(value) ?? 0,
    }));
    return { ...axis, values };
  });
}

/** The picked values of one axis, keyed `<axis>:<value>`. */
function pickedIn(axis: Axis, picked: Set<string>): string[] {
  return [...picked].flatMap((key) => {
    const at = key.indexOf(":");
    return key.slice(0, at) === axis.key ? [key.slice(at + 1)] : [];
  });
}

/** Every picked value has to hold, within an axis and across them: each one narrows. */
export function matchesAxes(entry: FixtureEntry, picked: Set<string>): boolean {
  if (picked.size === 0) return true;
  return AXES.every((axis) => {
    const wanted = pickedIn(axis, picked);
    if (wanted.length === 0) return true;
    const has = axis.of(entry);
    return wanted.every((value) => has.includes(value));
  });
}

/** The chip key the picked set and the deep link share. */
export function axisKey(axis: Pick<Axis, "key">, value: string): string {
  return `${axis.key}:${value}`;
}

/** One vocabulary hit: what typing in the filter box offers besides file names. */
export interface VocabHit {
  axis: AxisRow;
  value: AxisValue;
}

/**
 * Axis values whose name contains `needle`, so typing `alp` or `front` offers the chip
 * rather than searching a thousand file names for a word that is not in any of them.
 */
export function searchVocabulary(
  rows: AxisRow[],
  needle: string,
  picked: Set<string> = new Set(),
): VocabHit[] {
  const text = needle.trim().toLowerCase();
  if (text === "") return [];
  return rows.flatMap((axis) =>
    axis.values
      .filter(
        (value) =>
          value.value.toLowerCase().includes(text) &&
          !picked.has(axisKey(axis, value.value)),
      )
      .map((value) => ({ axis, value })),
  );
}

/** The rows of one section, for the sheet and the coverage view. */
export function sectionRows(rows: AxisRow[], section: Section): AxisRow[] {
  return rows.filter((row) => row.section === section);
}

/** The rows that sit above the sections, shown whichever section is open. */
export function pinnedRows(rows: AxisRow[]): AxisRow[] {
  return rows.filter((row) => row.section === null);
}

/** Values v2 defines that no fixture in the index carries: the gaps worth filling. */
export function coverageGaps(rows: AxisRow[]): VocabHit[] {
  return rows.flatMap((axis) =>
    axis.values
      .filter((value) => value.total === 0)
      .map((value) => ({ axis, value })),
  );
}

/** Key of a fixture in the index, and the value of the `fixture` deep link. */
export function fixtureKey(entry: FixtureEntry): string {
  return `${entry.directory}/${entry.name}`;
}

export async function loadFixtureIndex(): Promise<FixtureEntry[]> {
  const response = await fetch("fixtures.json");
  if (!response.ok)
    throw new Error(`fixtures.json: ${response.status} ${response.statusText}`);
  return (await response.json()) as FixtureEntry[];
}

export async function loadFixture(key: string): Promise<Uint8Array> {
  const response = await fetch(`fixtures/${key}`);
  if (!response.ok)
    throw new Error(`${key}: ${response.status} ${response.statusText}`);
  return new Uint8Array(await response.arrayBuffer());
}

/**
 * The address `raw` names, or null for one no tile could be fetched from.
 *
 * Relative reads against the page, so a tile beside it can be named as one. Only the two
 * web schemes come back: a link is something a stranger can hand over, and `file:` or
 * `data:` in it would ask the app to open something it was never pointed at.
 *
 * Nothing at all is not an address either: resolved against the page it would come back as
 * the page, and `?url=` would have the app fetch its own HTML and read it as a tile.
 */
export function tileAddress(raw: string): string | null {
  const text = raw.trim();
  if (text === "") return null;
  try {
    const address = new URL(text, location.href);
    const web = address.protocol === "http:" || address.protocol === "https:";
    return web ? address.href : null;
  } catch {
    return null;
  }
}

/**
 * Fetches a tile from anywhere, without credentials: an address a link handed over is not
 * one to send this reader's cookies to.
 */
export async function loadTile(address: string): Promise<Uint8Array> {
  const response = await fetch(address, { credentials: "omit" });
  if (!response.ok)
    throw new Error(`${address}: ${response.status} ${response.statusText}`);
  return new Uint8Array(await response.arrayBuffer());
}

/** What the picker's two column buttons sort on. */
export type SortKey = "name" | "bytes";

/** Sorted copy of `entries`; name falls back to the key so a tie is still stable. */
export function sortFixtures(
  entries: FixtureEntry[],
  key: SortKey,
  descending: boolean,
): FixtureEntry[] {
  const direction = descending ? -1 : 1;
  return [...entries].sort((a, b) => {
    const by =
      key === "bytes"
        ? a.bytes - b.bytes || compare(fixtureKey(a), fixtureKey(b))
        : compare(fixtureKey(a), fixtureKey(b));
    return by * direction;
  });
}

/** One `<optgroup>` of the Source picker. */
export interface FixtureGroup {
  label: string;
  entries: FixtureEntry[];
}

/** Everything up to a name's first underscore, which is what the synthetics vary within. */
export function fixturePrefix(name: string): string {
  return name.replace(/\.mlt$/, "").split("_")[0];
}

/** The picker's `<dir>/<prefix>` groups, ordered directory, then prefix, then name. */
export function groupFixtures(entries: FixtureEntry[]): FixtureGroup[] {
  const groups = new Map<string, FixtureEntry[]>();
  const sorted = [...entries].sort(
    (a, b) =>
      compare(a.directory, b.directory) ||
      compare(fixturePrefix(a.name), fixturePrefix(b.name)) ||
      compare(a.name, b.name),
  );
  for (const entry of sorted) {
    const label = `${entry.directory}/${fixturePrefix(entry.name)}`;
    const group = groups.get(label);
    if (group) group.push(entry);
    else groups.set(label, [entry]);
  }
  return [...groups].map(([label, group]) => ({ label, entries: group }));
}

function compare(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

/**
 * fzf-style subsequence match, so `pstrf` finds `props_str_fsst`.
 * A thousand-odd fixtures are named in abbreviations, which are quicker to half-remember than to spell.
 */
export function fuzzyMatch(haystack: string, needle: string): boolean {
  let at = 0;
  for (const char of needle) {
    at = haystack.indexOf(char, at) + 1;
    if (at === 0) return false;
  }
  return true;
}

/** One of the few tiles the empty state offers by hand, ahead of the whole index. */
export interface Starter {
  key: string;
  title: string;
  note: string;
}

const STARTERS: Starter[] = [
  {
    key: "0x02/point.mlt",
    title: "a single point",
    note: "the smallest tile there is",
  },
  {
    key: "0x02/mix_5_pt_line_poly_mpt_mline.mlt",
    title: "five geometry kinds",
    note: "point through multi-line, in one layer",
  },
  {
    key: "0x02/props_str_fsst.mlt",
    title: "compressed strings",
    note: "an FSST-coded property column",
  },
  {
    key: "0x02/nested_struct.mlt",
    title: "nested properties",
    note: "a struct column and its field tree",
  },
];

/** The starters this index carries, so a renamed fixture drops out instead of 404ing. */
export function starterFixtures(index: FixtureEntry[]): Starter[] {
  const keys = new Set(index.map(fixtureKey));
  return STARTERS.filter((starter) => keys.has(starter.key));
}
