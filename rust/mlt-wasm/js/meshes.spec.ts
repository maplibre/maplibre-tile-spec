import { readdirSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import {
  decodeTileColumns,
  MltGeometryType,
  mValueSlice,
  toElevation,
} from "./columns";
import { featureGeometry, geometryStarts } from "./featureGeometry";
import { tileGeoJson } from "./geojson";
import { decodeTile, decodeTile3D } from "./vectorTile";

const MESHES = new URL("../../../test/fixtures/meshes/tiles/", import.meta.url);
const SYNTHETIC_V2 = new URL("../../../test/synthetic/0x02/", import.meta.url);
const TILES = readdirSync(fileURLToPath(MESHES), { recursive: true })
  .map(String)
  .filter((path) => path.endsWith(".mlt"))
  .map((path) => path.replaceAll("\\", "/"))
  .sort();

type Xyz = [number, number, number];
type Tri = [number, number, number];

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
      expect(geometry.layout).toBe("meshes");
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
      expect(layer.vertexValues).toEqual([]);
      expect(layer.triangleValues.map((c) => c.name)).toEqual(["part"]);
      const [part] = layer.triangleValues;
      if (part?.type !== "string") throw new Error("column type");
      expect(part.present).toBeUndefined();
      expect(new Set(part.dictionary).size).toBe(part.dictionary.length);
      expect(part.codes.length).toBe(
        expected.flatMap((f) => f.triangles).length,
      );
      const parts = [...part.codes].map((code) => part.dictionary[code]);
      expect(parts).toEqual(expected.flatMap((f) => f.properties["m:part"]));
      expected.forEach((feature, f) => {
        expect(mValueSlice(part, f)).toEqual(feature.properties["m:part"]);
      });
    });

    it("reads each mesh's vertices, triangles and parts through decodeTile", () => {
      const layer2d = decodeTile(data).layers.buildings;
      expect([layer2d.zStep, layer2d.heightMode]).toEqual([
        -2,
        "relativeToGround",
      ]);
      expect(layer2d.vertexValues).toEqual([]);
      expect(layer2d.triangleValues.map((c) => c.name)).toEqual(["part"]);
      expected.forEach((feature, f) => {
        const decoded = layer2d.feature(f);
        expect(decoded.id).toBe(feature.id);
        expect(decoded.heightMode).toBe("relativeToGround");
        expect(decoded.properties.gml_id).toBe(feature.properties.gml_id);
        expect(decoded.loadGeometry()[0]?.map((p) => [p.x, p.y])).toEqual(
          feature.geometry.coordinates.map(([x, y]) => [x, y]),
        );
        expect(decoded.loadZ()).toEqual(
          feature.geometry.coordinates.map(([, , z]) => z),
        );
        expect(triples(decoded.loadTriangles())).toEqual(feature.triangles);
        expect(decoded.loadTriangleValues("part")).toEqual(
          feature.properties["m:part"],
        );
        expect(decoded.loadVertexValues("part")).toBeUndefined();
        expect(decoded.loadTriangleValues("missing")).toBeUndefined();
        expect(() => decoded.loadPolygons()).toThrow(/is a mesh/);
      });
    });

    it("reads each mesh's vertices, triangles and parts through decodeTile3D", () => {
      const layer3d = decodeTile3D(data).layers.buildings;
      expect([layer3d.zStep, layer3d.heightMode]).toEqual([
        -2,
        "relativeToGround",
      ]);
      expect(layer3d.vertexValues).toEqual([]);
      expect(layer3d.triangleValues.map((c) => c.name)).toEqual(["part"]);
      expected.forEach((feature, f) => {
        const decoded = layer3d.feature(f);
        expect(decoded.id).toBe(feature.id);
        expect(decoded.heightMode).toBe("relativeToGround");
        expect(decoded.properties.gml_id).toBe(feature.properties.gml_id);
        expect(decoded.loadGeometry()).toEqual([feature.geometry.coordinates]);
        expect(triples(decoded.loadTriangles())).toEqual(feature.triangles);
        expect(decoded.loadTriangleValues("part")).toEqual(
          feature.properties["m:part"],
        );
        expect(decoded.loadVertexValues("part")).toBeUndefined();
        expect(decoded.loadTriangleValues("missing")).toBeUndefined();
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
    "mvalues_str_dict.mlt",
    "z_mvalues.mlt",
  ];

  it.each(CASES)("matches tileGeoJson per feature on %s", (file) => {
    const data = new Uint8Array(readFileSync(new URL(file, SYNTHETIC_V2)));
    const [layer] = decodeTileColumns(data).layers;
    const geo = tileGeoJson(data);
    expect(layer.triangleValues).toEqual([]);
    for (const col of layer.vertexValues) {
      for (let f = 0; f < layer.featureCount; f++) {
        const expected = geo.features[f]?.properties?.[`m:${col.name}`] as
          | number[]
          | undefined;
        if (col.type === "bool") {
          const slice = mValueSlice(col, f);
          expect(slice && Array.from(slice, Boolean)).toEqual(expected);
        } else if (col.type === "f32") {
          const slice = mValueSlice(col, f);
          expect(slice?.length).toBe(expected?.length);
          slice?.forEach((v, i) => {
            expect(v).toBeCloseTo(expected?.[i] ?? Number.NaN, 5);
          });
        } else {
          const slice = mValueSlice(col, f);
          expect(slice && [...slice]).toEqual(expected);
        }
      }
    }
  });

  it("rejects feature indices outside the column", () => {
    const data = new Uint8Array(
      readFileSync(new URL("z_mvalues.mlt", SYNTHETIC_V2)),
    );
    const [layer] = decodeTileColumns(data).layers;
    const [dist] = layer.vertexValues;
    expect(dist?.name).toBe("dist");
    if (dist === undefined) return;
    expect(() => mValueSlice(dist, -1)).toThrow(RangeError);
    expect(() => mValueSlice(dist, layer.featureCount)).toThrow(RangeError);
  });

  it("reads a line's vertex values and no triangle values through decodeTile", () => {
    const data = new Uint8Array(
      readFileSync(new URL("z_mvalues.mlt", SYNTHETIC_V2)),
    );
    const layer2d = decodeTile(data).layers.layer1;
    expect(layer2d.triangleValues).toEqual([]);
    const line2d = layer2d.feature(0);
    expect(() => line2d.loadTriangles()).toThrow(/is not a mesh/);
    expect(line2d.loadVertexValues("dist")).toEqual(
      new Uint32Array([0, 10, 25]),
    );
    expect(line2d.loadTriangleValues("dist")).toBeUndefined();
  });

  it("reads a line's vertex values and no triangle values through decodeTile3D", () => {
    const data = new Uint8Array(
      readFileSync(new URL("z_mvalues.mlt", SYNTHETIC_V2)),
    );
    const layer3d = decodeTile3D(data).layers.layer1;
    expect(layer3d.triangleValues).toEqual([]);
    const line3d = layer3d.feature(0);
    expect(() => line3d.loadTriangles()).toThrow(/is not a mesh/);
    expect(line3d.loadVertexValues("dist")).toEqual(
      new Uint32Array([0, 10, 25]),
    );
    expect(line3d.loadTriangleValues("dist")).toBeUndefined();
  });
});

describe("Meshes edge cases", () => {
  const hex = (bytes: string) => new Uint8Array(Buffer.from(bytes, "hex"));

  it("reads an empty Meshes layer as one", () => {
    const [layer] = decodeTileColumns(
      hex("1f02096c616e646d61726b73f6000e0000000025000902050104666163658000"),
    ).layers;
    expect([layer.featureCount, layer.geometry.layout]).toEqual([0, "meshes"]);
    expect(layer.triangleValues.map((c) => c.name)).toEqual(["face"]);
    expect(geometryStarts(layer.geometry).featureGeometries).toEqual(
      Uint32Array.of(0),
    );
  });

  it("reads a mesh without vertices and one without triangles", () => {
    const data = hex(
      "2902096c616e646d61726b7376020e180200043002020080008000290609020206020206010018020002",
    );
    const [layer] = decodeTileColumns(data).layers;
    const meshes = [0, 1].map((f) => featureGeometry(layer, f));
    expect(
      meshes.map(
        (m) => m.kind === "mesh" && [m.vertices.length, m.triangles.length],
      ),
    ).toEqual([
      [0, 0],
      [6, 0],
    ]);
    const tile = decodeTile3D(data).layers.landmarks;
    expect([0, 1].map((f) => tile.feature(f).loadTriangles().length)).toEqual([
      0, 0,
    ]);
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
