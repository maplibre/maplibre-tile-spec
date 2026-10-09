import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { getTestCases } from "../../../test/synthetic/synthetic-test-utils";
import {
  decodeTileColumns,
  type MltColumnLayer,
  MltGeometryType,
} from "./columns";
import {
  featureGeometry,
  geometryStarts,
  isTrianglesOnly,
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
const closed = (ring: Position[]) =>
  ring.length > 0 ? [...ring, ring[0]] : ring;

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
          triangles.push([
            closed([...g.triangles.subarray(t, t + 3)].map((i) => all[i])),
          ]);
        }
        return { type: "MultiPolygon", coordinates: triangles };
      }
      const polygons = g.polygons.map((polygon) => {
        const all = positions(polygon.vertices, dimension);
        const starts = [0, ...polygon.holeIndices, all.length];
        return starts
          .slice(1)
          .map((end, k) => closed(all.slice(starts[k], end)));
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
      const { layers } = decodeTileColumns(
        new Uint8Array(readFileSync(fileName)),
      );
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
    expect(next).toBe(
      layer.geometry.vertices.length / layer.geometry.dimension,
    );
  });

  it("gives an outlined layer's triangles as the triangles-only layout stores them", () => {
    const outlined = featureGeometry(onlyLayer("0x02/z_poly_hole_tes"), 0);
    const bare = featureGeometry(onlyLayer("0x02/z_poly_hole_tri"), 0);
    if (outlined.kind !== "polygon" || bare.kind !== "polygon")
      throw new Error("not polygons");
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
    if (ringOffsets === undefined)
      throw new Error("poly_hole has no ring offsets");
    const backwards = Uint32Array.from(ringOffsets);
    [backwards[1], backwards[2]] = [backwards[2], backwards[1]];
    const broken: MltColumnLayer = {
      ...layer,
      geometry: { ...layer.geometry, ringOffsets: backwards },
    };
    expect(() => featureGeometry(broken, 0)).toThrow(/ends before it starts/);
  });

  it("rejects triangles whose offsets stop before the feature", () => {
    for (const name of ["0x02/z_poly_hole_tri", "0x02/z_poly_hole_tes"]) {
      const layer = onlyLayer(name);
      const { triangleOffsets } = layer.geometry;
      if (triangleOffsets === undefined)
        throw new Error(`${name} has no triangle offsets`);
      const cut = (offsets: Uint32Array | undefined): MltColumnLayer => ({
        ...layer,
        geometry: { ...layer.geometry, triangleOffsets: offsets },
      });
      expect(() =>
        featureGeometry(cut(triangleOffsets.subarray(0, 1)), 0),
      ).toThrow(/do not cover polygon 0/);
      expect(() => featureGeometry(cut(undefined), 0)).toThrow(
        /do not cover polygon 0/,
      );
    }
  });
});

/** Each geometry's and each ring's vertex runs of feature `f`, from `featureGeometry`. */
function featureRuns(
  layer: MltColumnLayer,
  f: number,
): { geometries: number[][]; rings: number[][] } {
  const g = featureGeometry(layer, f);
  const { dimension } = layer.geometry;
  const run = (r: { firstVertex: number; vertices: Int32Array }) => [
    r.firstVertex,
    r.firstVertex + r.vertices.length / dimension,
  ];
  switch (g.kind) {
    case "point": {
      const points = Array.from(
        { length: g.vertices.length / dimension },
        (_, i) => [g.firstVertex + i, g.firstVertex + i + 1],
      );
      return { geometries: points, rings: points };
    }
    case "line": {
      // A line is one geometry, and in a layer with rings one ring.
      const lines = g.lines.map(run);
      return { geometries: lines, rings: lines };
    }
    case "polygon":
      return {
        geometries: g.polygons.map(run),
        rings: g.polygons.flatMap((p) => {
          const [a, b] = run(p);
          const starts = [a, ...p.holeIndices.map((h) => a + h), b];
          return starts.slice(1).map((end, k) => [starts[k], end]);
        }),
      };
  }
}

describe("geometryStarts against featureGeometry on every synthetic fixture", () => {
  for (const { name, fileName } of getTestCases([]).active) {
    it(name, () => {
      const { layers } = decodeTileColumns(
        new Uint8Array(readFileSync(fileName)),
      );
      for (const layer of layers) {
        const { geometry } = layer;
        if (isTrianglesOnly(geometry)) {
          expect(() => geometryStarts(geometry)).toThrow(/TessPolygons/);
          continue;
        }
        const starts = geometryStarts(geometry);
        const {
          featureGeometries,
          geometryVertices,
          geometryRings,
          ringVertices,
        } = starts;
        const between = (a: Uint32Array, i: number, j: number) =>
          Array.from({ length: j - i }, (_, k) => [a[i + k], a[i + k + 1]]);
        for (let f = 0; f < layer.featureCount; f++) {
          const [g0, g1] = [featureGeometries[f], featureGeometries[f + 1]];
          const expected = featureRuns(layer, f);
          expect(between(geometryVertices, g0, g1)).toEqual(
            expected.geometries,
          );
          if (geometryRings !== undefined && ringVertices !== undefined) {
            const [r0, r1] = [geometryRings[g0], geometryRings[g1]];
            expect(between(ringVertices, r0, r1)).toEqual(expected.rings);
          }
        }
        expect(geometryVertices.at(-1)).toBe(
          geometry.vertices.length / geometry.dimension,
        );
      }
    });
  }
});

describe("geometryStarts", () => {
  it("resolves a layer once, and returns the same arrays on every call", () => {
    const { geometry } = onlyLayer("0x01/poly_hole");
    const first = geometryStarts(geometry);
    expect(geometryStarts(geometry)).toBe(first);
    expect(geometryStarts({ ...geometry })).not.toBe(first);
  });

  it("returns the stored offset columns as they are", () => {
    const { geometry } = onlyLayer("0x01/poly_hole");
    const starts = geometryStarts(geometry);
    expect(starts.geometryRings).toBe(geometry.partOffsets);
    expect(starts.ringVertices).toBe(geometry.ringOffsets);
  });

  it("returns a layer without rings' stored columns as they are, and no rings", () => {
    // A line, a multipoint and a multiline: geometry and part offsets, no ring offsets.
    const { geometry } = onlyLayer("0x01/mix_3_line_mpt_mline");
    const starts = geometryStarts(geometry);
    expect(starts.featureGeometries).toBe(geometry.geometryOffsets);
    expect(starts.geometryVertices).toBe(geometry.partOffsets);
    expect(starts.geometryRings).toBeUndefined();
    expect(starts.ringVertices).toBeUndefined();
  });

  it("rejects offset levels that start late, run backwards or stop short, and torn vertices", () => {
    const layer = onlyLayer("0x01/poly_hole");
    const { ringOffsets } = layer.geometry;
    if (ringOffsets === undefined)
      throw new Error("poly_hole has no ring offsets");
    const backwards = Uint32Array.from(ringOffsets);
    [backwards[1], backwards[2]] = [backwards[2], backwards[1]];
    expect(() =>
      geometryStarts({ ...layer.geometry, ringOffsets: backwards }),
    ).toThrow(/ends before it starts/);
    const late = Uint32Array.from(ringOffsets);
    late[0] = 1;
    expect(() =>
      geometryStarts({ ...layer.geometry, ringOffsets: late }),
    ).toThrow(/ringVertices starts at 1, expected 0/);
    const short = ringOffsets.subarray(0, ringOffsets.length - 1);
    expect(() =>
      geometryStarts({ ...layer.geometry, ringOffsets: short }),
    ).toThrow(/ringVertices has/);
    const { vertices, dimension } = layer.geometry;
    const torn = vertices.subarray(0, vertices.length - 1);
    expect(() => geometryStarts({ ...layer.geometry, vertices: torn })).toThrow(
      /not a multiple of its dimension/,
    );
    const fewer = vertices.subarray(0, vertices.length - dimension);
    expect(() =>
      geometryStarts({ ...layer.geometry, vertices: fewer }),
    ).toThrow(/ringVertices ends at \d+, expected/);
  });
});
