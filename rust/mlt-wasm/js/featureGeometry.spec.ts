import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { getTestCases } from "../../../test/synthetic/synthetic-test-utils";
import { decodeTileColumns, type MltColumnLayer, MltGeometryType } from "./columns";
import {
  toElevation,
  featureGeometry,
  type MltFeatureGeometry,
} from "./featureGeometry";

function onlyLayer(name: string): MltColumnLayer {
  const url = new URL(`../../../test/synthetic/${name}.mlt`, import.meta.url);
  const { layers } = decodeTileColumns(new Uint8Array(readFileSync(url)));
  expect(layers).toHaveLength(1);
  return layers[0];
}

type Position = number[];

/** The vertices of a view as positions, `dimension` numbers each. */
function positions(vertices: Int32Array, dimension: number): Position[] {
  const out: Position[] = [];
  for (let i = 0; i < vertices.length; i += dimension) {
    out.push(Array.from(vertices.subarray(i, i + dimension)));
  }
  return out;
}

/** A ring closed by repeating its first position, as GeoJSON writes it. */
const closed = (ring: Position[]) => (ring.length > 0 ? [...ring, ring[0]] : ring);

/** The GeoJSON geometry the synthetic fixtures expect, rebuilt from a feature view. */
function geoJson(g: MltFeatureGeometry, dimension: number): GeoJSON.Geometry {
  switch (g.kind) {
    case "point": {
      const points = positions(g.vertices, dimension);
      return g.type === MltGeometryType.Point
        ? { type: "Point", coordinates: points[0] }
        : { type: "MultiPoint", coordinates: points };
    }
    case "line": {
      const lines = g.lines.map((line) => positions(line.vertices, dimension));
      return g.type === MltGeometryType.LineString
        ? { type: "LineString", coordinates: lines[0] }
        : { type: "MultiLineString", coordinates: lines };
    }
    case "polygon": {
      if (g.polygons.length === 0 && g.triangles) {
        // Triangles without outlines: each triangle is a polygon of its own.
        const all = positions(g.vertices, dimension);
        const triangles: Position[][][] = [];
        for (let t = 0; t < g.triangles.length; t += 3) {
          triangles.push([closed([...g.triangles.subarray(t, t + 3)].map((i) => all[i]))]);
        }
        return { type: "MultiPolygon", coordinates: triangles };
      }
      const polygons = g.polygons.map((polygon) => {
        const all = positions(polygon.vertices, dimension);
        const starts = [0, ...polygon.holeIndices, all.length];
        return starts.slice(1).map((end, k) => closed(all.slice(starts[k], end)));
      });
      return g.type === MltGeometryType.Polygon
        ? { type: "Polygon", coordinates: polygons[0] }
        : { type: "MultiPolygon", coordinates: polygons };
    }
  }
}

describe("featureGeometry against every synthetic fixture", () => {
  for (const { name, content, fileName } of getTestCases([]).active) {
    it(name, () => {
      const expected = (content as GeoJSON.FeatureCollection).features;
      const { layers } = decodeTileColumns(new Uint8Array(readFileSync(fileName)));
      const actual = layers.flatMap((layer) =>
        Array.from({ length: layer.featureCount }, (_, i) =>
          geoJson(featureGeometry(layer, i), layer.geometry.dimension),
        ),
      );
      expect(actual).toEqual(expected.map((f) => f.geometry));
    });
  }
});

describe("featureGeometry", () => {
  it("places each feature in the layer's vertex sequence", () => {
    // A line, a multipoint and a multipolygon in one layer use every offset level.
    const layer = onlyLayer("0x01/mix_3_line_mpt_mline");
    let next = 0;
    for (let i = 0; i < layer.featureCount; i++) {
      const g = featureGeometry(layer, i);
      expect(g.firstVertex).toBe(next);
      next += g.vertices.length / layer.geometry.dimension;
    }
    expect(next).toBe(layer.geometry.vertices.length / layer.geometry.dimension);
  });

  it("gives an outlined layer's triangles as the triangles-only layout stores them", () => {
    const outlined = featureGeometry(onlyLayer("0x02/z_poly_hole_tes"), 0);
    const bare = featureGeometry(onlyLayer("0x02/z_poly_hole_tri"), 0);
    if (outlined.kind !== "polygon" || bare.kind !== "polygon") throw new Error("not polygons");
    const corners = (g: typeof outlined) => {
      const all = positions(g.vertices, 3);
      return Array.from(g.triangles ?? [], (i) => all[i]);
    };
    expect(outlined.polygons).toHaveLength(1);
    expect(bare.polygons).toEqual([]);
    expect(corners(outlined)).toEqual(corners(bare));
  });

  it("rejects an index that is not a feature", () => {
    const layer = onlyLayer("0x01/point");
    expect(() => featureGeometry(layer, 1)).toThrow(RangeError);
    expect(() => featureGeometry(layer, -1)).toThrow(RangeError);
  });

  it("rejects an offset level that runs backwards", () => {
    const layer = onlyLayer("0x01/poly_hole");
    const { ringOffsets } = layer.geometry;
    if (ringOffsets === undefined) throw new Error("poly_hole has no ring offsets");
    const backwards = Uint32Array.from(ringOffsets);
    [backwards[1], backwards[2]] = [backwards[2], backwards[1]];
    const broken: MltColumnLayer = { ...layer, geometry: { ...layer.geometry, ringOffsets: backwards } };
    expect(() => featureGeometry(broken, 0)).toThrow(/ends before it starts/);
  });

  it("rejects triangles whose offsets stop before the feature", () => {
    for (const name of ["0x02/z_poly_hole_tri", "0x02/z_poly_hole_tes"]) {
      const layer = onlyLayer(name);
      const { triangleOffsets } = layer.geometry;
      if (triangleOffsets === undefined) throw new Error(`${name} has no triangle offsets`);
      const cut = (offsets: Uint32Array | undefined): MltColumnLayer => ({
        ...layer,
        geometry: { ...layer.geometry, triangleOffsets: offsets },
      });
      expect(() => featureGeometry(cut(triangleOffsets.subarray(0, 1)), 0)).toThrow(/do not cover polygon 0/);
      expect(() => featureGeometry(cut(undefined), 0)).toThrow(/do not cover polygon 0/);
    }
  });
});

describe("toElevation", () => {
  it("offsets by -10000 m and scales by the step", () => {
    expect(toElevation(10000, 0)).toBe(0);
    expect(toElevation(21000, 0)).toBe(11000);
    expect(toElevation(1012, 1)).toBe(120);
  });

  it("keeps a fine grid's decimals exact", () => {
    expect(toElevation(1001234, -2)).toBe(12.34);
    expect(toElevation(10012345, -3)).toBe(12.345);
  });
});
