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

/** The picker's `<dir> · <prefix>` groups, ordered directory, then prefix, then name. */
export function groupFixtures(entries: FixtureEntry[]): FixtureGroup[] {
  const groups = new Map<string, FixtureEntry[]>();
  const sorted = [...entries].sort(
    (a, b) =>
      compare(a.directory, b.directory) ||
      compare(fixturePrefix(a.name), fixturePrefix(b.name)) ||
      compare(a.name, b.name),
  );
  for (const entry of sorted) {
    const label = `${entry.directory} · ${fixturePrefix(entry.name)}`;
    const group = groups.get(label);
    if (group) group.push(entry);
    else groups.set(label, [entry]);
  }
  return [...groups].map(([label, group]) => ({ label, entries: group }));
}

function compare(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}
