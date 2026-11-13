import { readdirSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import {
  decodeTileColumns,
  type MltColumnLayer,
  type MltGeometryColumns,
  MltGeometryType,
  type MltMValueColumn,
  toElevation,
} from "./columns";
import { featureGeometry, geometryStarts, isMeshes, isTrianglesOnly } from "./featureGeometry";
import { tileGeoJson } from "./geojson";
import { decodeTile3D } from "./vectorTile";

/** Written in `test/fixtures/meshes` by `mlt convert buildings.gml tiles --mlt-version 2 --min-zoom 15 --max-zoom 16`. */
const MESHES = new URL("../../../test/fixtures/meshes/tiles/", import.meta.url);
const TILES = readdirSync(fileURLToPath(MESHES), { recursive: true })
  .map(String)
  .filter((path) => path.endsWith(".mlt"))
  .map((path) => path.replaceAll("\\", "/"))
  .sort();

type Xyz = [number, number, number];
/** A triangle's three vertex indices. */
type Tri = [number, number, number];

/** A mesh feature as `tileGeoJson` writes it, through mlt-core's rows rather than its columns. */
interface MeshFeature {
  id: number;
  properties: { gml_id: string; "m:part": string[] };
  geometry: { type: "MultiPoint"; coordinates: Xyz[] };
  triangles: Tri[];
}

function triples(words: ArrayLike<number>): [number, number, number][] {
  const out: [number, number, number][] = [];
  for (let i = 0; i < words.length; i += 3) out.push([words[i], words[i + 1], words[i + 2]]);
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
      expect(geometry.dimension === 3 && [geometry.zStep, geometry.heightMode]).toEqual([
        -2,
        "relativeToGround",
      ]);
      expect([...geometry.types]).toEqual(expected.map(() => MltGeometryType.Mesh));
      expect(geometry.meshes).toBe(true);
    });

    it("has every feature's vertices and triangles", () => {
      expected.forEach((feature, f) => {
        const mesh = featureGeometry(layer, f);
        expect(mesh.kind).toBe("mesh");
        expect(triples(mesh.vertices)).toEqual(feature.geometry.coordinates);
        expect(mesh.kind === "mesh" && triples(mesh.triangles)).toEqual(feature.triangles);
      });
    });

    it("has a part per triangle", () => {
      const part = mValue(layer, "part");
      if (part.type !== "string") throw new Error("column type");
      expect(part.scope).toBe("triangle");
      expect(part.present).toBeUndefined();
      expect(new Set(part.dictionary).size).toBe(part.dictionary.length);
      expect(part.codes.length).toBe(expected.flatMap((f) => f.triangles).length);
      const parts = [...part.codes].map((code) => part.dictionary[code]);
      expect(parts).toEqual(expected.flatMap((f) => f.properties["m:part"]));
    });

    it("reads the same through decodeTile3D", () => {
      const layer3d = decodeTile3D(data).layers.buildings;
      expect([layer3d.zStep, layer3d.heightMode]).toEqual([-2, "relativeToGround"]);
      expect(layer3d.mValues.map((c) => c.name)).toEqual(["part"]);
      expected.forEach((feature, f) => {
        const decoded = layer3d.feature(f);
        expect(decoded.id).toBe(feature.id);
        expect(decoded.properties.gml_id).toBe(feature.properties.gml_id);
        expect(decoded.loadGeometry()).toEqual([feature.geometry.coordinates]);
        expect(triples(decoded.loadTriangles())).toEqual(feature.triangles);
      });
    });
  });
});

describe("a Meshes layer without features", () => {
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

  it("is still a Meshes layer, not a TessPolygons one", () => {
    expect(isMeshes(geometry)).toBe(true);
    expect(isTrianglesOnly(geometry)).toBe(false);
    expect(() => geometryStarts(geometry)).not.toThrow();
  });
});

describe("toElevation", () => {
  it("reads a height with no offset on fine and coarse grids", () => {
    expect(toElevation(1234, -2)).toBe(12.34);
    expect(toElevation(12, 1)).toBe(120);
  });
});
