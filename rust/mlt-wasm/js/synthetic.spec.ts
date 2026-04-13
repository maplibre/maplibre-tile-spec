import { readFile } from "node:fs/promises";
import type { VectorTileLike } from "@maplibre/vt-pbf";
import { describe, expect, it } from "vitest";
import {
  compareWithTolerance,
  getTestCases,
  writeActualOutput,
} from "../../../test/synthetic/synthetic-test-utils";
import {
  decodeTile,
  MltFeature,
  MltGeometryType,
  MltLayer,
} from "./vectorTile";

const UNIMPLEMENTED_SYNTHETICS = new Map([
  ["poly_collinear_fpf", "FastPFor not supported"],
  ["poly_collinear_fpf_tes", "FastPFor not supported"],
  ["poly_fpf", "FastPFor not supported"],
  ["poly_fpf_tes", "FastPFor not supported"],
  ["poly_hole_fpf", "FastPFor not supported"],
  ["poly_hole_fpf_tes", "FastPFor not supported"],
  ["poly_hole_touching_fpf", "FastPFor not supported"],
  ["poly_hole_touching_fpf_tes", "FastPFor not supported"],
  ["poly_multi_fpf", "FastPFor not supported"],
  ["poly_multi_fpf_tes", "FastPFor not supported"],
  ["poly_self_intersect_fpf", "FastPFor not supported"],
  ["poly_self_intersect_fpf_tes", "FastPFor not supported"],
]);

describe("MLT WASM Decoder - Synthetic tests", () => {
  expect.addEqualityTesters([compareWithTolerance]);
  const testCases = getTestCases(Array.from(UNIMPLEMENTED_SYNTHETICS.keys()));

  for (const { name, content, fileName } of testCases.active) {
    it(name, async () => {
      const actual = await decodeMLT(fileName);
      writeActualOutput(fileName, actual);
      expect(actual).toEqual(content);
    });
  }

  for (const skippedTest of testCases.skipped) {
    it.skip(skippedTest, () => {
      // Test is skipped since it is not supported yet
    });
  }
});

async function decodeMLT(
  mltFilePath: string,
): Promise<Record<string, unknown>> {
  const mltBuffer = await readFile(mltFilePath);
  const tile = decodeTile(new Uint8Array(mltBuffer));
  return tileToFeatureCollection(tile) as unknown as Record<string, unknown>;
}

function tileToFeatureCollection(
  tile: VectorTileLike,
): GeoJSON.FeatureCollection {
  const features: GeoJSON.Feature[] = [];
  for (const layer of Object.values(tile.layers)) {
    const mltLayer = layer as MltLayer;

    for (let i = 0; i < mltLayer.length; i++) {
      const feature = mltLayer.feature(i);
      const properties: Record<string, number | string | boolean> = {
        _layer: mltLayer.name,
        _extent: mltLayer.extent,
      };

      for (let k = 0; k < mltLayer.propertyKeys.length; k++) {
        const key = mltLayer.propertyKeys[k];
        const col = mltLayer.propertyColumns[k];
        let val = feature.properties[key];

        if (typeof val === "number") {
          if (Number.isNaN(val)) {
            if (col instanceof Float32Array) val = "f32::NAN";
            else if (col instanceof Float64Array) val = "f64::NAN";
          } else if (val === Infinity) {
            if (col instanceof Float32Array) val = "f32::INFINITY";
            else if (col instanceof Float64Array) val = "f64::INFINITY";
          } else if (val === -Infinity) {
            if (col instanceof Float32Array) val = "f32::NEG_INFINITY";
            else if (col instanceof Float64Array) val = "f64::NEG_INFINITY";
          }
        }
        if (val !== undefined) {
          properties[key] = val;
        }
      }

      const geojsonFeature: GeoJSON.Feature = {
        type: "Feature",
        geometry: getGeometry(feature),
        properties,
      };
      if (feature.id !== undefined) {
        geojsonFeature.id = feature.id;
      }
      features.push(geojsonFeature);
    }
  }
  return { type: "FeatureCollection", features };
}

function getGeometry(feature: MltFeature): GeoJSON.Geometry {
  const rings = feature.loadGeometry();
  const coords = rings.map((ring) => ring.map((p) => [p.x, p.y]));
  switch (feature.mltType) {
    case MltGeometryType.Point:
      return { type: "Point", coordinates: coords[0][0] };
    case MltGeometryType.MultiPoint:
      return { type: "MultiPoint", coordinates: coords.map((r) => r[0]) };
    case MltGeometryType.LineString:
      return { type: "LineString", coordinates: coords[0] };
    case MltGeometryType.MultiLineString:
      return { type: "MultiLineString", coordinates: coords };
    case MltGeometryType.Polygon: {
      const polygons = feature.loadPolygons();
      return {
        type: "Polygon",
        coordinates: polygons[0].map((ring) =>
          closeRing(ring.map((p) => [p.x, p.y])),
        ),
      };
    }
    case MltGeometryType.MultiPolygon: {
      const polygons = feature.loadPolygons();
      return {
        type: "MultiPolygon",
        coordinates: polygons.map((polygon) =>
          polygon.map((ring) =>
            closeRing(ring.map((p) => [p.x, p.y])),
          ),
        ),
      };
    }
    default:
      throw new Error(`Unsupported MLT geometry type: ${feature.mltType}`);
  }
}

/** GeoJSON polygons must have their first and last coordinate identical. */
function closeRing(ring: number[][]): number[][] {
  if (ring.length === 0) return ring;
  const first = ring[0];
  const last = ring[ring.length - 1];
  if (first[0] !== last[0] || first[1] !== last[1]) {
    return [...ring, [first[0], first[1]]];
  }
  return ring;
}

