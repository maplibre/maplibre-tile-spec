import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { getTestCases } from "../../../test/synthetic/synthetic-test-utils";
import {
  decodeTile,
  decodeTile3D,
  type MltFeature3D,
  MltGeometryType,
  type MltLayer,
  type MltLayer3D,
} from "./vectorTile";

type Expected = {
  features: { geometry: GeoJSON.Geometry }[];
};

/** Fixtures written with the TessPolygonsWithOutlines (`_tes`) or TessPolygons (`_tri`) layout. */
const TESSELLATED = /_(tes|tri)$/;

describe("decodeTile3D against every synthetic fixture", () => {
  const { active } = getTestCases([]);
  for (const { name, content, fileName } of active) {
    const data = new Uint8Array(readFileSync(fileName));
    const layers2d = Object.values(decodeTile(data).layers) as MltLayer[];
    if (layers2d.some((layer) => layer.zStep === undefined)) {
      it(`${name} (a flat layer: throws)`, () => {
        expect(() => decodeTile3D(data)).toThrow(/has no z coordinates/);
      });
      continue;
    }
    if (TESSELLATED.test(name)) {
      it(`${name} (tessellated: throws)`, () => {
        expect(() => decodeTile3D(data)).toThrow(
          /uses the TessPolygons(WithOutlines)? geometry layout/,
        );
      });
      continue;
    }
    it(name, () => {
      const expected = (content as Expected).features;
      let next = 0;
      for (const layer of Object.values(decodeTile3D(data).layers)) {
        for (let i = 0; i < layer.length; i++) {
          const { geometry } = expected[next++];
          expectClose(
            coordinates(layer.feature(i)),
            inMetres(geometry, layer.zStep),
          );
        }
      }
      expect(next).toBe(expected.length);
    });
  }
});

describe("tessellated layouts", () => {
  it.each([
    ["0x02/z_poly_hole_tes", "TessPolygonsWithOutlines"],
    ["0x02/z_poly_hole_tri", "TessPolygons"],
  ])("%s is rejected as %s", (fixture, layout) => {
    expect(() => layersOf(fixture)).toThrow(
      `layer "layer1" uses the ${layout} geometry layout, which decodeTile3D does not support`,
    );
  });
});

describe("elevation", () => {
  it("puts z_point 12 m up", () => {
    const [layer] = layersOf("0x02/z_point");
    expect(layer.zStep).toBe(-1);
    expect(layer.feature(0).loadGeometry()[0][0][2]).toBeCloseTo(12, 9);
  });

  it("reads z_step_finest in millimetres", () => {
    const [layer] = layersOf("0x02/z_step_finest");
    const feature = layer.feature(0);
    const z = feature.loadZ();
    feature
      .loadGeometry()
      .flat()
      .forEach(([, , e], v) => {
        expect(e).toBeCloseTo((z[v] - 10_000_000) / 1000, 9);
      });
  });
});

function layersOf(fixture: string): MltLayer3D[] {
  const url = new URL(
    `../../../test/synthetic/${fixture}.mlt`,
    import.meta.url,
  );
  return Object.values(decodeTile3D(readFileSync(url)).layers);
}

/** The fixture's geometry with each raw z replaced by its elevation, an oracle independent of the decoder. */
function inMetres(geometry: GeoJSON.Geometry, zStep: number): unknown {
  const convert = (value: unknown): unknown => {
    if (!Array.isArray(value)) return value;
    if (typeof value[0] === "number") {
      const [x, y, z] = value as number[];
      return [x, y, -10000 + z * 10 ** zStep];
    }
    return value.map(convert);
  };
  if (geometry.type === "GeometryCollection") throw new Error("not in MLT");
  return convert(geometry.coordinates);
}

/** Feature coordinates in GeoJSON nesting; polygon rings come back closed, as GeoJSON wants. */
function coordinates(feature: MltFeature3D): unknown {
  switch (feature.mltType) {
    case MltGeometryType.Point:
      return feature.loadGeometry()[0][0];
    case MltGeometryType.MultiPoint:
      return feature.loadGeometry().map((r) => r[0]);
    case MltGeometryType.LineString:
      return feature.loadGeometry()[0];
    case MltGeometryType.MultiLineString:
      return feature.loadGeometry();
    case MltGeometryType.Polygon:
      return feature.loadPolygons()[0];
    case MltGeometryType.MultiPolygon:
      return feature.loadPolygons();
    default:
      throw new Error(`unknown geometry type ${feature.mltType}`);
  }
}

function expectClose(actual: unknown, expected: unknown): void {
  if (typeof expected === "number") {
    expect(actual).toBeCloseTo(expected, 6);
    return;
  }
  expect(Array.isArray(actual)).toBe(true);
  const a = actual as unknown[];
  const e = expected as unknown[];
  expect(a).toHaveLength(e.length);
  for (let i = 0; i < e.length; i++) expectClose(a[i], e[i]);
}
