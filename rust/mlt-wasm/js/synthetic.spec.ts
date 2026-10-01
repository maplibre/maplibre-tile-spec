import { readdirSync } from "node:fs";
import { readFile } from "node:fs/promises";
import type { VectorTileLike } from "@maplibre/vt-pbf";
import { describe, expect, it } from "vitest";
import {
  compareWithTolerance,
  expectUnsupported,
  getTestCases,
  writeActualOutput,
} from "../../../test/synthetic/synthetic-test-utils";
import {
  decodeTile,
  type MltFeature,
  MltGeometryType,
  type MltLayer,
} from "./vectorTile";

const V2_GAPS: [RegExp, string][] = [
  [/^(z_)?mvalues(?!_all_null$)/, "the vector-tile API has no m-value columns"],
  [/^nested_/, "the vector-tile API has no nested columns"],
  [/_tri$/, "a triangles-only layer has no offsets for loadGeometry to walk"],
];

const UNIMPLEMENTED_SYNTHETICS = new Map(
  readdirSync(new URL("../../../test/synthetic/0x02", import.meta.url))
    .filter((file) => file.endsWith(".mlt"))
    .map((file) => file.slice(0, -".mlt".length))
    .flatMap((name) => {
      const gap = V2_GAPS.find(([pattern]) => pattern.test(name));
      return gap ? [[`0x02/${name}`, gap[1]] as const] : [];
    }),
);

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

  for (const { name, content, fileName } of testCases.skipped) {
    it(`${name} (unsupported: ${reasonFor(name)})`, () =>
      expectUnsupported(() => decodeMLT(fileName), content));
  }
});

/**
 * Skip-list entries name either a single fixture or a whole directory, matching `getTestCases`.
 */
function reasonFor(name: string): string {
  for (const [entry, reason] of UNIMPLEMENTED_SYNTHETICS) {
    if (name === entry || name.startsWith(`${entry}/`)) return reason;
  }
  return "not supported";
}

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
      const properties: Record<string, number | string | boolean | number[]> = {
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

      if (feature.zStep !== undefined) {
        properties._z_step = feature.zStep;
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
  const z = feature.zStep === undefined ? undefined : feature.loadZ();
  let next = 0;
  const position = (p: { x: number; y: number }): number[] =>
    z === undefined ? [p.x, p.y] : [p.x, p.y, z[next++]];
  const lines = () => feature.loadGeometry().map((ring) => ring.map(position));
  switch (feature.mltType) {
    case MltGeometryType.Point:
      return { type: "Point", coordinates: lines()[0][0] };
    case MltGeometryType.MultiPoint:
      return { type: "MultiPoint", coordinates: lines().map((r) => r[0]) };
    case MltGeometryType.LineString:
      return { type: "LineString", coordinates: lines()[0] };
    case MltGeometryType.MultiLineString:
      return { type: "MultiLineString", coordinates: lines() };
    case MltGeometryType.Polygon: {
      const polygons = feature.loadPolygons();
      return {
        type: "Polygon",
        coordinates: polygons[0].map((ring) => closeRing(ring.map(position))),
      };
    }
    case MltGeometryType.MultiPolygon: {
      const polygons = feature.loadPolygons();
      return {
        type: "MultiPolygon",
        coordinates: polygons.map((polygon) =>
          polygon.map((ring) => closeRing(ring.map(position))),
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
    return [...ring, [...first]];
  }
  return ring;
}
