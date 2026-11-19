import type { Feature, FeatureCollection } from "geojson";
import { describe, expect, it } from "vitest";
import type {
  MltColumnLayer,
  MltColumnTile,
  MltMValueColumn,
} from "./annotate.ts";
import {
  extentOf,
  factsOf,
  hueOf,
  layerOf,
  layersOf,
  runsOf,
  triangleHues,
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

type Corner = [number, number, number];

const MLT_GEOMETRY_TYPE_MESH = 6;

interface Mesh {
  vertices: Corner[];
  triangles: Corner[];
}

const mesh = (triangles: number): Mesh => ({
  vertices: Array.from({ length: triangles }, (_, i): Corner[] => [
    [i, 0, 0],
    [i, 1, 0],
    [i + 1, 0, 5],
  ]).flat(),
  triangles: Array.from(
    { length: triangles },
    (_, i): Corner => [i * 3, i * 3 + 1, i * 3 + 2],
  ),
});

function strings(
  name: string,
  perMesh: (string[] | undefined)[],
): MltMValueColumn {
  const dictionary = [...new Set(perMesh.flatMap((values) => values ?? []))];
  const offsets = [0];
  for (const values of perMesh)
    offsets.push(offsets[offsets.length - 1] + (values?.length ?? 0));
  return {
    name,
    type: "string",
    offsets: Uint32Array.from(offsets),
    codes: Uint32Array.from(
      perMesh
        .flatMap((values) => values ?? [])
        .map((value) => dictionary.indexOf(value)),
    ),
    dictionary,
  };
}

function meshLayer(
  name: string,
  meshes: Mesh[],
  triangleValues: MltMValueColumn[] = [],
  { extent = 4096, zStep = 0 } = {},
): MltColumnLayer {
  const geometryOffsets = [0];
  const triangleOffsets = [0];
  const indices: number[] = [];
  for (const { vertices, triangles } of meshes) {
    const base = geometryOffsets[geometryOffsets.length - 1];
    indices.push(...triangles.flat().map((corner) => base + corner));
    geometryOffsets.push(base + vertices.length);
    triangleOffsets.push(
      triangleOffsets[triangleOffsets.length - 1] + triangles.length,
    );
  }
  return {
    name,
    extent,
    version: 2,
    featureCount: meshes.length,
    geometry: {
      types: new Uint8Array(meshes.length).fill(MLT_GEOMETRY_TYPE_MESH),
      layout: "meshes",
      dimension: 3,
      zStep,
      heightMode: "absolute",
      geometryOffsets: Uint32Array.from(geometryOffsets),
      vertices: Int32Array.from(meshes.flatMap((m) => m.vertices.flat())),
      triangleOffsets: Uint32Array.from(triangleOffsets),
      indexBuffer: Uint32Array.from(indices),
    },
    properties: [],
    vertexValues: [],
    triangleValues,
  };
}

const tileOf = (...layers: MltColumnLayer[]): MltColumnTile => ({ layers });

describe("triangleHues", () => {
  it("numbers triangle values tile-wide in first-seen order, so a value keeps its hue across meshes", () => {
    const houses = meshLayer(
      "houses",
      [mesh(3)],
      [strings("surface", [["roof", "wall", "roof"]])],
    );
    const sheds = meshLayer(
      "sheds",
      [mesh(2)],
      [strings("surface", [["ground", "wall"]])],
    );
    const hues = triangleHues(tileOf(houses, sheds));
    expect(hues.get(houses)).toEqual([0, 1, 0]);
    expect(hues.get(sheds)).toEqual([2, 1]);
  });

  it("cycles past the six palette hues", () => {
    const values = ["a", "b", "c", "d", "e", "f", "g"];
    const tower = meshLayer("tower", [mesh(7)], [strings("part", [values])]);
    expect(triangleHues(tileOf(tower)).get(tower)).toEqual([
      0, 1, 2, 3, 4, 5, 0,
    ]);
  });

  it("colours by a layer's first triangle column", () => {
    const house = meshLayer(
      "house",
      [mesh(2)],
      [
        strings("material", [["brick", "brick"]]),
        strings("surface", [["roof", "wall"]]),
      ],
    );
    expect(triangleHues(tileOf(house)).get(house)).toEqual([0, 0]);
  });

  it("gives the hue past the palette to the triangles of a mesh without values in the column", () => {
    const street = meshLayer(
      "street",
      [mesh(1), mesh(2)],
      [strings("surface", [undefined, ["roof", "wall"]])],
    );
    expect(triangleHues(tileOf(street)).get(street)).toEqual([6, 0, 1]);
  });

  it("leaves out a layer without a triangle column, and one that is not meshes", () => {
    const plain = meshLayer("plain", [mesh(2)]);
    const meshes = meshLayer(
      "outlines",
      [mesh(1)],
      [strings("surface", [["roof"]])],
    );
    const outlines = {
      ...meshes,
      geometry: { ...meshes.geometry, layout: "outlines" as const },
    };
    expect(triangleHues(tileOf(plain, outlines)).size).toBe(0);
  });
});
