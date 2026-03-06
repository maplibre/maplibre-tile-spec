import { readFile } from "node:fs/promises";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";
import type Point from "@mapbox/point-geometry";
import type { VectorTileFeatureLike, VectorTileLike } from "@maplibre/vt-pbf";
import { describe, expect, it } from "vitest";
import {
  compareWithTolerance,
  getTestCases,
  writeActualOutput,
} from "../../../test/synthetic/synthetic-test-utils";
import { decodeTile } from "./vectorTile";

const __dirname = dirname(fileURLToPath(import.meta.url));
const EARCUT_MAX_RINGS = 500;

// The WASM decoder handles F64, U32/U64 natively — skip list is much shorter
// than the TS decoder's.
const UNIMPLEMENTED_SYNTHETICS = new Map([
  ["poly_multi_morton_ring_morton", "decodes to zero coordinates"],
  ["poly_multi_morton_ring_no_morton", "decodes to zero coordinates"],
  ["poly_colinear_fpf", "FastPFor not supported"],
  ["poly_colinear_fpf_tes", "FastPFor not supported"],
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
  ["props_shared_dict", "Purposefully not supported until after the currently ongoing refactoring"],
  ["props_shared_dict_fsst", "Purposefully not supported until after the currently ongoing refactoring"],
  ["props_shared_dict_no_child_name", "Purposefully not supported until after the currently ongoing refactoring"],
  ["props_shared_dict_no_child_name_fsst", "Purposefully not supported until after the currently ongoing refactoring"],
  ["props_shared_dict_no_struct_name", "Purposefully not supported until after the currently ongoing refactoring"],
  ["props_shared_dict_no_struct_name_fsst", "Purposefully not supported until after the currently ongoing refactoring"],
  ["props_shared_dict_one_child", "Purposefully not supported until after the currently ongoing refactoring"],
  ["props_shared_dict_one_child_fsst", "Purposefully not supported until after the currently ongoing refactoring"],
  ["props_shared_dict", "Purposefully not supported until after the currently ongoing refactoring"]
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
    for (let i = 0; i < layer.length; i++) {
      const feature = layer.feature(i);
      const geojsonFeature: GeoJSON.Feature = {
        type: "Feature",
        geometry: getGeometry(feature),
        properties: {
          _layer: layer.name,
          _extent: layer.extent,
          ...feature.properties,
        },
      };
      if (feature.id !== undefined) {
        geojsonFeature.id = feature.id;
      }
      features.push(geojsonFeature);
    }
  }
  return { type: "FeatureCollection", features };
}

function getGeometry(feature: VectorTileFeatureLike): GeoJSON.Geometry {
  const rings = feature.loadGeometry();
  const coords = rings.map((ring) => ring.map((p) => [p.x, p.y]));
  switch (feature.type) {
    case 1:
      return rings.length === 1
        ? { type: "Point", coordinates: coords[0][0] }
        : { type: "MultiPoint", coordinates: coords.map((r) => r[0]) };
    case 2:
      return rings.length === 1
        ? { type: "LineString", coordinates: coords[0] }
        : { type: "MultiLineString", coordinates: coords };
    case 3: {
      const polygons = classifyRings(rings, EARCUT_MAX_RINGS);
      return polygons.length === 1
        ? {
            type: "Polygon",
            coordinates: polygons[0].map((ring) =>
              closeRing(ring.map((p) => [p.x, p.y])),
            ),
          }
        : {
            type: "MultiPolygon",
            coordinates: polygons.map((polygon) =>
              polygon.map((ring) => closeRing(ring.map((p) => [p.x, p.y]))),
            ),
          };
    }
    default:
      throw new Error(`Unsupported MVT geometry type: ${feature.type}`);
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

// Minimal classifyRings: groups MVT rings into polygon groups by winding order.
// Matches the @maplibre/maplibre-gl-style-spec implementation: the winding of
// the first ring determines which direction is "outer"; subsequent rings that
// wind the same way start new polygons, opposite-winding rings are holes.
function classifyRings(rings: Point[][], maxRings: number): Point[][][] {
  if (rings.length <= 1) return [rings];

  const polygons: Point[][][] = [];
  let currentPolygon: Point[][] | undefined;
  let outerCCW: boolean | undefined;

  for (const ring of rings) {
    const area = signedArea(ring);
    if (area === 0) continue;
    if (outerCCW === undefined) outerCCW = area < 0;
    if (area < 0 === outerCCW) {
      // same winding as the first ring → new outer ring
      if (maxRings > 0 && polygons.length === maxRings) break;
      currentPolygon = [ring];
      polygons.push(currentPolygon);
    } else {
      // opposite winding → hole
      currentPolygon?.push(ring);
    }
  }

  return polygons;
}

function signedArea(ring: Point[]): number {
  let sum = 0;
  for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
    sum += (ring[j].x - ring[i].x) * (ring[i].y + ring[j].y);
  }
  return sum;
}
