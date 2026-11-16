import { readdirSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import {
  columnValue,
  decodeTileColumns,
  type MltColumn,
  type MltColumnLayer,
  type MltGeometryColumns,
  MltGeometryType,
  type MltMValueColumn,
  mValueSlice,
  toElevation,
} from "./columns";
import {
  featureGeometry,
  geometryStarts,
  isMeshes,
  isTrianglesOnly,
} from "./featureGeometry";
import { tileGeoJson } from "./geojson";
import { decodeTile, decodeTile3D } from "./vectorTile";

/** Written by `mlt convert buildings.gml tiles --mlt-version 2 --min-zoom 15 --max-zoom 16`. */
const MESHES = new URL("../../../test/fixtures/meshes/tiles/", import.meta.url);
const SYNTHETIC_V2 = new URL("../../../test/synthetic/0x02/", import.meta.url);
const TILES = readdirSync(fileURLToPath(MESHES), { recursive: true })
  .map(String)
  .filter((path) => path.endsWith(".mlt"))
  .map((path) => path.replaceAll("\\", "/"))
  .sort();

type Xyz = [number, number, number];
/** A triangle's three vertex indices. */
type Tri = [number, number, number];

/** A mesh feature as `tileGeoJson` writes it, through mlt-core's rows. */
interface MeshFeature {
  id: number;
  properties: { gml_id: string; "m:part": string[] };
  geometry: { type: "MultiPoint"; coordinates: Xyz[] };
  triangles: Tri[];
}

function triples(words: ArrayLike<number>): [number, number, number][] {
  const out: [number, number, number][] = [];
  for (let i = 0; i < words.length; i += 3) {
    out.push([words[i], words[i + 1], words[i + 2]]);
  }
  return out;
}

function mValue(layer: MltColumnLayer, name: string): MltMValueColumn {
  const column = layer.mValues?.find((c) => c.name === name);
  if (column === undefined) throw new Error(`no m-value column ${name}`);
  return column;
}

describe("the mesh tiles", () => {
  it("are all here", () => {
    expect(TILES).toEqual(["15/17294/10590.mlt", "16/34588/21180.mlt"]);
  });

  describe.each(TILES)("%s", (tile) => {
    const data = new Uint8Array(readFileSync(new URL(tile, MESHES)));
    const expected = tileGeoJson(data).features as unknown as MeshFeature[];
    const [layer] = decodeTileColumns(data).layers;
    const { geometry } = layer;

    it("is one Meshes layer of heights on a 1 cm grid", () => {
      expect(decodeTileColumns(data).layers).toHaveLength(1);
      expect([layer.name, layer.version, layer.featureCount]).toEqual([
        "buildings",
        2,
        expected.length,
      ]);
      expect(geometry.dimension).toBe(3);
      expect(
        geometry.dimension === 3 && [geometry.zStep, geometry.heightMode],
      ).toEqual([-2, "relativeToGround"]);
      expect([...geometry.types]).toEqual(
        expected.map(() => MltGeometryType.Mesh),
      );
      expect(geometry.meshes).toBe(true);
    });

    it("has every feature's vertices and triangles", () => {
      expected.forEach((feature, f) => {
        const mesh = featureGeometry(layer, f);
        expect(mesh.kind).toBe("mesh");
        expect(triples(mesh.vertices)).toEqual(feature.geometry.coordinates);
        expect(mesh.kind === "mesh" && triples(mesh.triangles)).toEqual(
          feature.triangles,
        );
      });
    });

    it("has a part per triangle", () => {
      const part = mValue(layer, "part");
      if (part.type !== "string") throw new Error("column type");
      expect(part.scope).toBe("triangle");
      expect(part.present).toBeUndefined();
      expect(new Set(part.dictionary).size).toBe(part.dictionary.length);
      expect(part.codes.length).toBe(
        expected.flatMap((f) => f.triangles).length,
      );
      const parts = [...part.codes].map((code) => part.dictionary[code]);
      expect(parts).toEqual(expected.flatMap((f) => f.properties["m:part"]));
      expected.forEach((feature, f) => {
        expect(mValueSlice(layer, part, f)).toEqual(
          feature.properties["m:part"],
        );
      });
    });

    it("reads the same through decodeTile3D and exposes heightMode in 2D", () => {
      const layer2d = decodeTile(data).layers.buildings;
      expect([layer2d.zStep, layer2d.heightMode]).toEqual([
        -2,
        "relativeToGround",
      ]);
      expect(layer2d.feature(0).heightMode).toBe("relativeToGround");
      expect(() => layer2d.feature(0).loadPolygons()).toThrow(/is a mesh/);

      const layer3d = decodeTile3D(data).layers.buildings;
      expect([layer3d.zStep, layer3d.heightMode]).toEqual([
        -2,
        "relativeToGround",
      ]);
      expect(layer3d.mValues.map((c) => c.name)).toEqual(["part"]);
      expected.forEach((feature, f) => {
        const decoded = layer3d.feature(f);
        expect(decoded.id).toBe(feature.id);
        expect(decoded.heightMode).toBe("relativeToGround");
        expect(decoded.properties.gml_id).toBe(feature.properties.gml_id);
        expect(decoded.loadGeometry()).toEqual([feature.geometry.coordinates]);
        expect(triples(decoded.loadTriangles())).toEqual(feature.triangles);
        expect(decoded.loadMValues("part")).toEqual(
          feature.properties["m:part"],
        );
        expect(decoded.loadMValues("missing")).toBeUndefined();
        expect(() => decoded.loadPolygons()).toThrow(/is a mesh/);
      });
    });
  });
});

describe("synthetic v2 m-value columns", () => {
  const CASES = [
    "mvalues.mlt",
    "mvalues_all_null.mlt",
    "mvalues_bool.mlt",
    "mvalues_i8.mlt",
    "mvalues_u8.mlt",
    "mvalues_i32.mlt",
    "mvalues_u32.mlt",
    "mvalues_i64.mlt",
    "mvalues_u64.mlt",
    "mvalues_f32_dict.mlt",
    "mvalues_f64_alp.mlt",
    "mvalues_str_plain.mlt",
    "mvalues_str_fsst.mlt",
    "mvalues_str_dict.mlt",
    "z_mvalues.mlt",
  ];

  it.each(CASES)("matches tileGeoJson per feature on %s", (file) => {
    const data = new Uint8Array(readFileSync(new URL(file, SYNTHETIC_V2)));
    const [layer] = decodeTileColumns(data).layers;
    const geo = tileGeoJson(data);
    for (const col of layer.mValues ?? []) {
      expect(col.scope).toBe("vertex");
      for (let f = 0; f < layer.featureCount; f++) {
        const slice = mValueSlice(layer, col, f);
        const expected = geo.features[f]?.properties?.[`m:${col.name}`];
        if (expected === undefined) {
          expect(slice).toBeUndefined();
        } else if (col.type === "bool") {
          expect(Array.from(slice as Uint8Array, Boolean)).toEqual(expected);
        } else if (col.type === "f32") {
          const arr = Array.from(slice as Float32Array);
          expect(arr).toHaveLength((expected as number[]).length);
          arr.forEach((v, i) => {
            expect(v).toBeCloseTo((expected as number[])[i], 5);
          });
        } else {
          expect(Array.from(slice as ArrayLike<unknown>)).toEqual(expected);
        }
      }
    }
  });

  it("rejects m-value columns passed to columnValue and bad indices", () => {
    const data = new Uint8Array(
      readFileSync(new URL("z_mvalues.mlt", SYNTHETIC_V2)),
    );
    const [layer] = decodeTileColumns(data).layers;
    const dist = mValue(layer, "dist");
    expect(() => columnValue(dist as unknown as MltColumn, 0)).toThrow(
      TypeError,
    );
    expect(() => mValueSlice(layer, dist, -1)).toThrow(RangeError);
    expect(() => mValueSlice(layer, dist, layer.featureCount)).toThrow(
      RangeError,
    );

    const line3d = decodeTile3D(data).layers.layer1.feature(0);
    expect(() => line3d.loadTriangles()).toThrow(/is not a mesh/);
    expect(Array.from(line3d.loadMValues("dist") as Uint32Array)).toEqual([
      0, 10, 25,
    ]);
  });
});

describe("Meshes edge cases", () => {
  it("recognises an empty Meshes layer, not a TessPolygons one", () => {
    const geometry: MltGeometryColumns = {
      types: new Uint8Array(0),
      dimension: 3,
      zStep: -2,
      heightMode: "relativeToGround",
      meshes: true,
      geometryOffsets: new Uint32Array([0]),
      triangleOffsets: new Uint32Array([0]),
      indexBuffer: new Uint32Array(0),
      vertices: new Int32Array(0),
    };
    expect(isMeshes(geometry)).toBe(true);
    expect(isTrianglesOnly(geometry)).toBe(false);
    expect(() => geometryStarts(geometry)).not.toThrow();
  });

  it("reads a 0-vertex and a 0-triangle mesh feature", () => {
    const layer: MltColumnLayer = {
      name: "landmarks",
      extent: 4096,
      version: 2,
      featureCount: 2,
      geometry: {
        types: new Uint8Array([MltGeometryType.Mesh, MltGeometryType.Mesh]),
        dimension: 3,
        zStep: -2,
        heightMode: "relativeToGround",
        meshes: true,
        geometryOffsets: new Uint32Array([0, 0, 2]),
        triangleOffsets: new Uint32Array([0, 0, 0]),
        indexBuffer: new Uint32Array(0),
        vertices: new Int32Array([1, 2, 3, 4, 5, 6]),
      },
      properties: [],
    };
    const empty = featureGeometry(layer, 0);
    expect(
      empty.kind === "mesh" && [empty.vertices.length, empty.triangles.length],
    ).toEqual([0, 0]);
    const spare = featureGeometry(layer, 1);
    expect(
      spare.kind === "mesh" && [spare.vertices.length, spare.triangles.length],
    ).toEqual([6, 0]);
  });
});

describe("toElevation", () => {
  it("reads heights without producing -0", () => {
    expect(toElevation(1234, -2)).toBe(12.34);
    expect(toElevation(0, 0)).toBe(0);
    expect(Object.is(toElevation(-0, 0), 0)).toBe(true);
    expect(toElevation(12, 1)).toBe(120);
  });
});
