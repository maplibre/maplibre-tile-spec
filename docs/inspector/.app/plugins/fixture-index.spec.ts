import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { FIXTURE_DIRECTORIES, indexFixtures } from "./fixture-index.ts";

const index = indexFixtures(join(import.meta.dirname, "..", "fixtures"));

describe("indexFixtures", () => {
  it("indexes every synthetic tile", () => {
    expect(index).toHaveLength(1088);
  });

  it("indexes each directory", () => {
    const counts = Object.fromEntries(
      FIXTURE_DIRECTORIES.map((directory) => [
        directory,
        index.filter((entry) => entry.directory === directory).length,
      ]),
    );
    expect(counts).toEqual({
      "0x01": 366,
      "0x01-rust": 194,
      "0x02": 528,
    });
  });

  it("indexes tiles only", () => {
    expect(index.filter((entry) => !entry.name.endsWith(".mlt"))).toEqual([]);
  });

  it("records a non-zero size for every tile", () => {
    expect(index.filter((entry) => entry.bytes <= 0)).toEqual([]);
  });

  it("orders by directory then by name", () => {
    const keys = index.map((entry) => `${entry.directory}/${entry.name}`);
    const ordered = FIXTURE_DIRECTORIES.flatMap((directory) =>
      keys.filter((key) => key.startsWith(`${directory}/`)).sort(),
    );
    expect(keys).toEqual(ordered);
  });
});
