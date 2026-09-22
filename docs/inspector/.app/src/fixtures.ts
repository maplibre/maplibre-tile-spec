/** The synthetic fixture index the app is built against. */

/** One synthetic fixture, as the build-time index records it. */
export interface FixtureEntry {
  /** File name including the `.mlt` extension. */
  name: string;
  /** Directory under `test/synthetic` the fixture lives in. */
  directory: string;
  /** Size of the tile on disk. */
  bytes: number;
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

/** Underscore-separated words of a fixture name, which is what the synthetics vary in. */
export function fixtureTokens(name: string): string[] {
  return name.replace(/\.mlt$/, "").split("_");
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

/** One row of the sheet's quick filters, whose tags OR while the facets AND. */
export interface FixtureFacet {
  label: string;
  tags: string[];
}

export const FIXTURE_FACETS: FixtureFacet[] = [
  { label: "version", tags: ["v1", "v2"] },
  {
    label: "geometry",
    tags: ["point", "line", "polygon", "multi-part", "tessellated"],
  },
  {
    label: "encoding",
    tags: [
      "dict",
      "fsst",
      "rle",
      "delta",
      "fastpfor",
      "morton",
      "alp",
      "bit-packed",
      "plain",
    ],
  },
  {
    label: "column",
    tags: ["ids", "properties", "m-values", "nested", "nulls", "extent"],
  },
];

/**
 * Tags a name's tokens earn, taking the synthetics' abbreviations from the generator's own legend.
 * `polyh` is a polygon with a hole, `tes` carries tessellation triangles, `fpf` is FastPFor,
 * `bp` is bit-packed dictionary codes and `sp` is a presence bitfield shared between columns.
 */
const TOKEN_TAGS: Record<string, string[]> = {
  pt: ["point"],
  point: ["point"],
  mpt: ["point", "multi-part"],
  mpoint: ["point", "multi-part"],
  multipoint: ["point", "multi-part"],
  line: ["line"],
  mline: ["line", "multi-part"],
  multiline: ["line", "multi-part"],
  poly: ["polygon"],
  polyh: ["polygon"],
  mpoly: ["polygon", "multi-part"],
  multi: ["multi-part"],
  tes: ["tessellated"],
  dict: ["dict"],
  dictionary: ["dict"],
  fsst: ["fsst"],
  rle: ["rle"],
  delta: ["delta"],
  fpf: ["fastpfor"],
  morton: ["morton"],
  alp: ["alp"],
  bp: ["bit-packed"],
  plain: ["plain"],
  id: ["ids"],
  ids: ["ids"],
  id64: ["ids"],
  ids64: ["ids"],
  prop: ["properties"],
  props: ["properties"],
  mvalues: ["m-values"],
  nested: ["nested"],
  struct: ["nested"],
  map: ["nested"],
  list: ["nested"],
  null: ["nulls"],
  nulls: ["nulls"],
  presence: ["nulls"],
  sp: ["nulls"],
  extent: ["extent"],
};

/** Version a fixture directory holds, which its name carries ahead of any `-rust` or `-java` suffix. */
function versionTag(directory: string): string | null {
  if (directory.startsWith("0x01")) return "v1";
  if (directory.startsWith("0x02")) return "v2";
  return null;
}

/** The tags a fixture carries, which is what the sheet's chips filter on. */
export function fixtureTags(entry: FixtureEntry): Set<string> {
  const tags = new Set<string>();
  const version = versionTag(entry.directory);
  if (version !== null) tags.add(version);
  for (const token of fixtureTokens(entry.name)) {
    for (const tag of TOKEN_TAGS[token] ?? []) tags.add(tag);
  }
  return tags;
}

/** Whether a fixture passes the picked chips, which OR within a facet and AND across them. */
export function matchesTags(tags: Set<string>, picked: Set<string>): boolean {
  return FIXTURE_FACETS.every((facet) => {
    const wanted = facet.tags.filter((tag) => picked.has(tag));
    return wanted.length === 0 || wanted.some((tag) => tags.has(tag));
  });
}

/**
 * Fixtures each chip would leave, counted with its own facet's picks set aside.
 * A chip beside a picked sibling counts what picking it too would add, rather than nothing.
 */
export function tagCounts(
  entries: FixtureEntry[],
  picked: Set<string>,
): Map<string, number> {
  const tagged = entries.map(fixtureTags);
  const counts = new Map<string, number>();
  for (const facet of FIXTURE_FACETS) {
    const others = new Set(
      [...picked].filter((tag) => !facet.tags.includes(tag)),
    );
    for (const tag of facet.tags) {
      counts.set(
        tag,
        tagged.filter((tags) => tags.has(tag) && matchesTags(tags, others))
          .length,
      );
    }
  }
  return counts;
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
