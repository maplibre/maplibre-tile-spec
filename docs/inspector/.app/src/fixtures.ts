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
