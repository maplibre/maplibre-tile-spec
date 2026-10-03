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
      const layers = Object.values(decodeTile3D(data).layers);
      expect(layers.map((l) => l.zStep)).toEqual(layers2d.map((l) => l.zStep));
      let next = 0;
      for (const layer of layers) {
        for (let i = 0; i < layer.length; i++) {
          const { geometry } = expected[next++];
          if (geometry.type === "GeometryCollection")
            throw new Error("not in MLT");
          expect(coordinates(layer.feature(i))).toEqual(geometry.coordinates);
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

describe("raw z", () => {
  it("keeps z_point's z on its 1 dm grid, 12 m up", () => {
    const [layer] = layersOf("0x02/z_point");
    expect(layer.zStep).toBe(-1);
    const [x, y, z] = layer.feature(0).loadGeometry()[0][0];
    expect([x, y, z]).toEqual([13, 42, 100_120]);
    expect(-10000 + z * 10 ** layer.zStep).toBeCloseTo(12, 9);
  });
});

function layersOf(fixture: string): MltLayer3D[] {
  const url = new URL(
    `../../../test/synthetic/${fixture}.mlt`,
    import.meta.url,
  );
  return Object.values(decodeTile3D(readFileSync(url)).layers);
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
