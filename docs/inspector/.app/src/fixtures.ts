/** The fixture index the app is built against. */

/** One fixture tile, as the build-time index records it. */
export interface FixtureEntry {
  /** File name including the `.mlt` extension. */
  name: string;
  /** Directory under `test/synthetic` the fixture lives in. */
  directory: string;
  /** Size of the tile on disk. */
  bytes: number;
  /** Geometry types `mlt ls` found, absent when it could not read the tile. */
  geometries?: string[];
  /** Coarse encodings `mlt ls` found, named as the facet bar shows them. */
  encodings?: string[];
  /** What the tile holds, as `mlt ls` flags it: str, i32, bool, m-values... */
  content?: string[];
}

/** Wire tag of the fixture's directory, e.g. `0x02` for `0x02-rust`. */
export function fixtureTag(entry: FixtureEntry): string {
  return entry.directory.split("-")[0];
}

/** One row of filter buttons, and the entry field it reads. */
export interface Facet {
  label: string;
  /** Values in the order the bar shows them, each with the entries it matches. */
  values: { value: string; count: number }[];
  of: (entry: FixtureEntry) => string[];
}

const FACET_FIELDS: Omit<Facet, "values">[] = [
  { label: "tag", of: (e) => [fixtureTag(e)] },
  { label: "geometry", of: (e) => e.geometries ?? [] },
  { label: "encoding", of: (e) => e.encodings ?? [] },
  { label: "has", of: (e) => e.content ?? [] },
];

/**
 * The filter bar's buttons, counted over `entries`.
 * A value every entry carries cannot narrow anything, so it is left out.
 */
export function facetsOf(entries: FixtureEntry[]): Facet[] {
  return FACET_FIELDS.flatMap((field) => {
    const { of } = field;
    const counts = new Map<string, number>();
    for (const entry of entries)
      for (const value of of(entry))
        counts.set(value, (counts.get(value) ?? 0) + 1);
    const values = [...counts]
      .filter(([, count]) => count < entries.length)
      .sort((a, b) => b[1] - a[1] || compare(a[0], b[0]))
      .map(([value, count]) => ({ value, count }));
    return values.length > 0 ? [{ ...field, values }] : [];
  });
}

/** Every picked value has to hold, within a facet and across them: each one narrows. */
export function matchesFacets(
  entry: FixtureEntry,
  facets: Facet[],
  picked: Set<string>,
): boolean {
  return facets.every((facet) => {
    const wanted = facet.values
      .map(({ value }) => value)
      .filter((value) => picked.has(`${facet.label}:${value}`));
    if (wanted.length === 0) return true;
    const has = facet.of(entry);
    return wanted.every((value) => has.includes(value));
  });
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
