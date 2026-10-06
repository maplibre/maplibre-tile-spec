import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { getTestCases } from "../../../test/synthetic/synthetic-test-utils";
import { decodeTileColumns, type MltColumnLayer, MltGeometryType } from "./columns";
import {
  toElevation,
  featureGeometry,
  type MltFeatureGeometry,
  toLngLat,
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

describe("toLngLat", () => {
  const layer = onlyLayer("0x02/z_point");
  const tile = { z: 0, x: 0, y: 0 };

  it("converts tile coordinates and z, keeping the layout", () => {
    const [lng, lat, metres] = toLngLat(layer.geometry.vertices, layer, tile);
    expect(lng).toBeCloseTo((13 / layer.extent) * 360 - 180, 12);
    const my = 42 / layer.extent;
    expect(lat).toBeCloseTo((Math.atan(Math.sinh(Math.PI * (1 - 2 * my))) * 180) / Math.PI, 12);
    expect(metres).toBe(12);
  });

  it("puts the world's centre at the equator and the prime meridian", () => {
    const flat = onlyLayer("0x01/point");
    const centre = Int32Array.of(flat.extent / 2, flat.extent / 2);
    expect(Array.from(toLngLat(centre, flat, tile))).toEqual([0, 0]);
  });

  it("puts the world's north-west corner at Web Mercator's limit", () => {
    const flat = onlyLayer("0x01/point");
    const [lng, lat] = toLngLat(Int32Array.of(0, 0), flat, tile);
    expect(lng).toBe(-180);
    expect(lat).toBeCloseTo(85.0511287798, 9);
  });

  it("offsets by the tile's position", () => {
    const flat = onlyLayer("0x01/point");
    const corner = Int32Array.of(0, 0);
    const [lng, lat] = toLngLat(corner, flat, { z: 1, x: 1, y: 1 });
    expect(lng).toBe(0);
    expect(lat).toBeCloseTo(0, 12);
  });
});
