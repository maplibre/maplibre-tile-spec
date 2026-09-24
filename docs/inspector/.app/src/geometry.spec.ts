import type { Feature, FeatureCollection } from "geojson";
import { describe, expect, it } from "vitest";
import {
  extentOf,
  factsOf,
  hueOf,
  layerOf,
  layersOf,
  runsOf,
  vertexCount,
} from "./geometry.ts";

function feature(
  geometry: Feature["geometry"],
  properties: Record<string, unknown> = {},
): Feature {
  return { type: "Feature", geometry, properties };
}

const line = (...coordinates: [number, number][]): Feature["geometry"] => ({
  type: "LineString",
  coordinates,
});

function collection(...features: Feature[]): FeatureCollection {
  return { type: "FeatureCollection", features };
}

describe("extentOf", () => {
  it("takes the widest extent the tile declares", () => {
    const tile = collection(
      feature(line([0, 0]), { _extent: 64 }),
      feature(line([0, 0]), { _extent: 4096 }),
    );
    expect(extentOf(tile)).toBe(4096);
  });

  it("keeps a small extent rather than padding it out to the usual one", () => {
    expect(extentOf(collection(feature(line([0, 0]), { _extent: 64 })))).toBe(
      64,
    );
  });

  it("falls back for a tile that declares none", () => {
    expect(extentOf(collection())).toBe(4096);
  });
});

describe("layersOf", () => {
  it("names each layer once, in the order the tile carries them", () => {
    const tile = collection(
      feature(line([0, 0]), { _layer: "roads" }),
      feature(line([0, 0]), { _layer: "water" }),
      feature(line([0, 0]), { _layer: "roads" }),
    );
    expect(layersOf(tile)).toEqual(["roads", "water"]);
  });
});

describe("layerOf", () => {
  it("reads the layer the decoder tagged the feature with", () => {
    expect(layerOf(feature(line([0, 0]), { _layer: "roads" }))).toBe("roads");
  });
});

describe("vertexCount", () => {
  it("counts a point as one", () => {
    expect(vertexCount({ type: "Point", coordinates: [1, 2] })).toBe(1);
  });

  it("counts a line's own vertices", () => {
    expect(vertexCount(line([0, 0], [1, 1], [2, 2]))).toBe(3);
  });

  it("counts a polygon's rings together, holes included", () => {
    expect(
      vertexCount({
        type: "Polygon",
        coordinates: [
          [
            [0, 0],
            [4, 0],
            [4, 4],
            [0, 0],
          ],
          [
            [1, 1],
            [2, 1],
            [2, 2],
            [1, 1],
          ],
        ],
      }),
    ).toBe(8);
  });

  it("reaches through a multi-geometry's extra nesting", () => {
    expect(
      vertexCount({
        type: "MultiPolygon",
        coordinates: [
          [
            [
              [0, 0],
              [1, 0],
              [1, 1],
              [0, 0],
            ],
          ],
        ],
      }),
    ).toBe(4);
  });
});

describe("runsOf", () => {
  it("collapses a repeated value into one inclusive range", () => {
    expect(runsOf([5, 5, 5, 9, 9])).toEqual([
      { from: 0, to: 2, value: 5 },
      { from: 3, to: 4, value: 9 },
    ]);
  });

  it("gives a lone value a range of its own", () => {
    expect(runsOf([1, 2])).toEqual([
      { from: 0, to: 0, value: 1 },
      { from: 1, to: 1, value: 2 },
    ]);
  });

  it("starts a new run when a value comes back later", () => {
    expect(runsOf(["a", "b", "a"]).map((run) => run.from)).toEqual([0, 1, 2]);
  });

  it("has nothing to say about no values", () => {
    expect(runsOf([])).toEqual([]);
  });
});

describe("factsOf", () => {
  const facts = factsOf(
    feature(line([0, 0], [1, 1]), {
      _layer: "roads",
      _extent: 4096,
      name: "High St",
      "m:height": [3, 3, 7],
    }),
  );

  it("names the geometry and counts its vertices", () => {
    expect(facts.type).toBe("LineString");
    expect(facts.vertices).toBe(2);
    expect(facts.layer).toBe("roads");
  });

  it("leaves the decoder's own tags out of the properties", () => {
    expect(facts.properties).toEqual([["name", "High St"]]);
  });

  it("reads an m-prefixed column as vertex ranges rather than a property", () => {
    expect(facts.mValues).toEqual([
      {
        name: "height",
        runs: [
          { from: 0, to: 1, value: 3 },
          { from: 2, to: 2, value: 7 },
        ],
      },
    ]);
  });

  it("omits the count for a point, which always has exactly one vertex", () => {
    expect(
      factsOf(feature({ type: "Point", coordinates: [1, 2] })).vertices,
    ).toBeNull();
  });
});

describe("hueOf", () => {
  it("gives each geometry type its own slot in the block palette", () => {
    const types = [
      "Point",
      "MultiPoint",
      "LineString",
      "MultiLineString",
      "Polygon",
      "MultiPolygon",
    ] as const;
    const hues = types.map((type) =>
      hueOf({ type, coordinates: [] } as unknown as Feature["geometry"]),
    );
    expect(new Set(hues).size).toBe(types.length);
  });
});
