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
  hasMeshes,
  hueOf,
  layerOf,
  layersOf,
  meshColumns,
  meshScene,
  metresPerUnit,
  runsOf,
  vertexCount,
  widestExtent,
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
  { extent = 4096, zStep = 0, vertexValues = [] as MltMValueColumn[] } = {},
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
    vertexValues,
    triangleValues,
  };
}

const tileOf = (...layers: MltColumnLayer[]): MltColumnTile => ({ layers });

describe("meshColumns", () => {
  it("lists the triangle columns before the vertex columns of the meshes in scope, each once", () => {
    const house = meshLayer(
      "buildings",
      [mesh(1)],
      [strings("part", [["roof"]])],
      { vertexValues: [strings("shade", [["a", "b", "c"]])] },
    );
    const shed = meshLayer(
      "buildings",
      [mesh(1)],
      [strings("part", [["wall"]]), strings("material", [["wood"]])],
    );
    const tree = meshLayer("trees", [mesh(1)], [strings("species", [["oak"]])]);
    expect(meshColumns(tileOf(house, shed, tree), "buildings")).toEqual([
      { name: "part", scope: "triangle" },
      { name: "material", scope: "triangle" },
      { name: "shade", scope: "vertex" },
    ]);
  });

  it("leaves out the columns of a layer that is not meshes", () => {
    const meshes = meshLayer("roads", [mesh(1)], [strings("lane", [["a"]])]);
    const outlines = {
      ...meshes,
      geometry: { ...meshes.geometry, layout: "outlines" as const },
    };
    expect(meshColumns(tileOf(outlines), null)).toEqual([]);
  });
});

describe("widestExtent", () => {
  it("takes the widest extent of the tile's layers", () => {
    const tile = tileOf(
      meshLayer("fine", [], [], { extent: 8192 }),
      meshLayer("coarse", [], [], { extent: 64 }),
    );
    expect(widestExtent(tile)).toBe(8192);
  });

  it("falls back for a tile without layers", () => {
    expect(widestExtent(tileOf())).toBe(4096);
  });
});

describe("hasMeshes", () => {
  it("finds a Meshes layer in the whole tile, or under the scoped name", () => {
    const meshes = meshLayer("buildings", [mesh(1)]);
    const outlines = {
      ...meshLayer("roads", [mesh(1)]),
      geometry: { ...meshes.geometry, layout: "outlines" as const },
    };
    const tile = tileOf(outlines, meshes);
    expect(hasMeshes(tile, null)).toBe(true);
    expect(hasMeshes(tile, "buildings")).toBe(true);
    expect(hasMeshes(tile, "roads")).toBe(false);
    expect(hasMeshes(tileOf(outlines), null)).toBe(false);
  });
});

const SURFACE = { name: "surface", scope: "triangle" } as const;

describe("meshScene", () => {
  it("lays mesh triangles out as x, height in tile units, y, with each triangle's hue", () => {
    const house = meshLayer(
      "buildings",
      [mesh(2)],
      [strings("surface", [["roof", "wall"]])],
      { zStep: -1 },
    );
    const scene = meshScene(tileOf(house), null, 2, SURFACE);
    expect(Array.from(scene.positions)).toEqual([
      0, 0, 0, 0, 0, 1, 1, 0.25, 0, 1, 0, 0, 1, 0, 1, 2, 0.25, 0,
    ]);
    expect(Array.from(scene.hues)).toEqual([0, 0, 0, 1, 1, 1]);
    expect(scene.legend).toEqual([
      { value: "roof", hue: 0 },
      { value: "wall", hue: 1 },
    ]);
  });

  it("numbers values tile-wide in first-seen order, so a value keeps its hue across meshes", () => {
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
    const scene = meshScene(tileOf(houses, sheds), null, 1, SURFACE);
    expect(Array.from(scene.hues)).toEqual([
      0, 0, 0, 1, 1, 1, 0, 0, 0, 2, 2, 2, 1, 1, 1,
    ]);
  });

  it("cycles past the six palette hues", () => {
    const values = ["a", "b", "c", "d", "e", "f", "g"];
    const tower = meshLayer("tower", [mesh(7)], [strings("part", [values])]);
    const scene = meshScene(tileOf(tower), null, 1, {
      name: "part",
      scope: "triangle",
    });
    expect(Array.from(scene.hues).filter((_, at) => at % 3 === 0)).toEqual([
      0, 1, 2, 3, 4, 5, 0,
    ]);
  });

  it("colours by the picked triangle column", () => {
    const house = meshLayer(
      "house",
      [mesh(2)],
      [
        strings("material", [["brick", "brick"]]),
        strings("surface", [["roof", "wall"]]),
      ],
    );
    expect(Array.from(meshScene(tileOf(house), null, 1, SURFACE).hues)).toEqual(
      [0, 0, 0, 1, 1, 1],
    );
    expect(
      Array.from(
        meshScene(tileOf(house), null, 1, {
          name: "material",
          scope: "triangle",
        }).hues,
      ),
    ).toEqual([0, 0, 0, 0, 0, 0]);
  });

  it("colours each corner by its vertex in a vertex column", () => {
    const street = meshLayer("buildings", [mesh(1), mesh(1)], [], {
      vertexValues: [
        strings("shade", [
          ["dark", "light", "dark"],
          ["light", "light", "mid"],
        ]),
      ],
    });
    const scene = meshScene(tileOf(street), null, 1, {
      name: "shade",
      scope: "vertex",
    });
    expect(Array.from(scene.hues)).toEqual([0, 1, 0, 1, 1, 2]);
    expect(scene.legend.map((entry) => entry.value)).toEqual([
      "dark",
      "light",
      "mid",
    ]);
  });

  it("gives the hue past the palette to the triangles of a mesh without values in the column", () => {
    const street = meshLayer(
      "street",
      [mesh(1), mesh(2)],
      [strings("surface", [undefined, ["roof", "wall"]])],
    );
    expect(
      Array.from(meshScene(tileOf(street), null, 1, SURFACE).hues),
    ).toEqual([6, 6, 6, 0, 0, 0, 1, 1, 1]);
  });

  it("reads each mesh's triangles from the layer-wide index buffer", () => {
    const street = meshLayer("buildings", [mesh(1), mesh(1)]);
    const scene = meshScene(tileOf(street), null, 1, null);
    expect(Array.from(scene.positions)).toEqual([
      0, 0, 0, 0, 0, 1, 1, 5, 0, 0, 0, 0, 0, 0, 1, 1, 5, 0,
    ]);
  });

  it("leaves out layers that are not meshes, and layers outside the scoped one", () => {
    const house = meshLayer("buildings", [mesh(1)]);
    const shed = meshLayer("sheds", [mesh(1)]);
    const outline = {
      ...meshLayer("outlines", [mesh(1)]),
      geometry: { ...house.geometry, layout: "outlines" as const },
    };
    expect(
      meshScene(tileOf(house, shed, outline), "sheds", 1, null).positions,
    ).toEqual(new Float32Array([0, 0, 0, 0, 0, 1, 1, 5, 0]));
    expect(meshScene(tileOf(outline), null, 1, null).positions.length).toBe(0);
  });

  it("brings every layer onto the widest extent, heights included", () => {
    const fine = meshLayer("fine", [mesh(1)], [], { extent: 8192 });
    const coarse = meshLayer("coarse", [mesh(1)], [], { extent: 4096 });
    const scene = meshScene(tileOf(fine, coarse), null, 0.5, null);
    expect(Array.from(scene.positions)).toEqual([
      0, 0, 0, 0, 0, 1, 1, 10, 0, 0, 0, 0, 0, 0, 2, 2, 10, 0,
    ]);
  });

  it("gives every triangle the hue past the palette when no column is picked", () => {
    const house = meshLayer(
      "buildings",
      [mesh(1)],
      [strings("surface", [["roof"]])],
    );
    const scene = meshScene(tileOf(house), null, 1, null);
    expect(Array.from(scene.hues)).toEqual([6, 6, 6]);
    expect(scene.legend).toEqual([]);
  });
});

describe("metresPerUnit", () => {
  it("reads the tile's latitude and zoom from a z/x/y address", () => {
    expect(
      metresPerUnit("http://localhost:5181/16/33561/21645.mlt", 4096),
    ).toBeCloseTo(0.0918986, 6);
  });

  it("reads the tile from the path of an address with a query", () => {
    expect(
      metresPerUnit("https://host/16/33561/21645.mlt?token=abc#top", 4096),
    ).toBeCloseTo(0.0918986, 6);
  });

  it("takes a unit as a metre when the address names no tile", () => {
    expect(metresPerUnit("0x02/mesh_roof_wall", 64)).toBe(1);
    expect(metresPerUnit(null, 4096)).toBe(1);
  });
});
