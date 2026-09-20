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
