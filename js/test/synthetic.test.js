import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";
import {
  MltGeometryType,
  decodeTile,
  tileGeoJson,
} from "../src/index.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const SYNTHETIC_DIR = path.resolve(__dirname, "../../test/synthetic");

const RELATIVE_FLOAT_TOLERANCE = 0.0001 / 100;
const ABSOLUTE_FLOAT_TOLERANCE = 1.1920928955078125e-7; // f32::EPSILON

function collectMltFiles(dir, prefix = "") {
  const out = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const rel = prefix ? `${prefix}/${entry.name}` : entry.name;
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      out.push(...collectMltFiles(full, rel));
    } else if (entry.isFile() && entry.name.endsWith(".mlt")) {
      out.push({
        name: rel.slice(0, -".mlt".length),
        mltPath: full,
        jsonPath: full.slice(0, -".mlt".length) + ".json",
      });
    }
  }
  return out.sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0));
}

function parseSentinelNumber(v) {
  if (typeof v === "string") {
    if (v.endsWith("NAN")) return Number.NaN;
    if (v.endsWith("NEG_INFINITY")) return Number.NEGATIVE_INFINITY;
    if (v.endsWith("INFINITY")) return Number.POSITIVE_INFINITY;
  }
  return v;
}

function valuesEqual(actual, expected, atPath = "$") {
  const a = parseSentinelNumber(actual);
  const b = parseSentinelNumber(expected);

  if (typeof a === "number" && typeof b === "number") {
    if (!Number.isFinite(b)) {
      if (!Object.is(a, b)) {
        throw new Error(`${atPath}: expected ${expected}, got ${actual}`);
      }
      return;
    }
    if (Math.abs(b) < ABSOLUTE_FLOAT_TOLERANCE) {
      if (Math.abs(a) > ABSOLUTE_FLOAT_TOLERANCE) {
        throw new Error(`${atPath}: expected ~0 (${b}), got ${a}`);
      }
      return;
    }
    const relErr = Math.abs(a - b) / Math.abs(b);
    if (relErr > RELATIVE_FLOAT_TOLERANCE) {
      throw new Error(`${atPath}: expected ${b}, got ${a} (relErr=${relErr})`);
    }
    return;
  }

  if (Array.isArray(a) && Array.isArray(b)) {
    if (a.length !== b.length) {
      throw new Error(
        `${atPath}: array length mismatch (expected ${b.length}, got ${a.length})`,
      );
    }
    for (let i = 0; i < a.length; i++) {
      valuesEqual(a[i], b[i], `${atPath}[${i}]`);
    }
    return;
  }

  if (
    a !== null &&
    b !== null &&
    typeof a === "object" &&
    typeof b === "object" &&
    !Array.isArray(a) &&
    !Array.isArray(b)
  ) {
    const keysA = Object.keys(a).sort();
    const keysB = Object.keys(b).sort();
    assert.deepEqual(
      keysA,
      keysB,
      `${atPath}: object keys mismatch (${keysA.join(",")} vs ${keysB.join(",")})`,
    );
    for (const k of keysA) {
      valuesEqual(a[k], b[k], `${atPath}.${k}`);
    }
    return;
  }

  if (!Object.is(a, b)) {
    throw new Error(
      `${atPath}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`,
    );
  }
}

const V2_VT_GAPS = [
  /^(z_)?mvalues(?!_all_null$)/,
  /^nested_/,
  /_tri$/,
];

function vectorTileToFeatureCollection(tile) {
  const features = [];
  for (const layer of Object.values(tile.layers)) {
    for (let i = 0; i < layer.length; i++) {
      const feature = layer.feature(i);
      const properties = {
        _layer: layer.name,
        _extent: layer.extent,
      };
      Object.assign(properties, feature.properties);
      if (feature.zStep !== undefined) {
        properties._z_step = feature.zStep;
      }
      const z = feature.zStep === undefined ? undefined : feature.loadZ();
      let next = 0;
      const position = (p) =>
        z === undefined ? [p.x, p.y] : [p.x, p.y, z[next++]];
      const lines = () =>
        feature.loadGeometry().map((ring) => ring.map(position));
      const ringCoordinates = (ring) => {
        const coords = ring.slice(0, -1).map(position);
        return coords.length === 0 ? coords : [...coords, [...coords[0]]];
      };
      let geometry;
      switch (feature.mltType) {
        case MltGeometryType.Point:
          geometry = { type: "Point", coordinates: lines()[0][0] };
          break;
        case MltGeometryType.MultiPoint:
          geometry = {
            type: "MultiPoint",
            coordinates: lines().map((r) => r[0]),
          };
          break;
        case MltGeometryType.LineString:
          geometry = { type: "LineString", coordinates: lines()[0] };
          break;
        case MltGeometryType.MultiLineString:
          geometry = { type: "MultiLineString", coordinates: lines() };
          break;
        case MltGeometryType.Polygon:
          geometry = {
            type: "Polygon",
            coordinates: feature.loadPolygons()[0].map(ringCoordinates),
          };
          break;
        case MltGeometryType.MultiPolygon:
          geometry = {
            type: "MultiPolygon",
            coordinates: feature
              .loadPolygons()
              .map((poly) => poly.map(ringCoordinates)),
          };
          break;
        default:
          throw new Error(`Unsupported MLT geometry type ${feature.mltType}`);
      }
      const out = { type: "Feature", properties, geometry };
      if (feature.id !== undefined) out.id = feature.id;
      features.push(out);
    }
  }
  return { type: "FeatureCollection", features };
}

const ALL_CASES = collectMltFiles(SYNTHETIC_DIR);

describe("Pure JS MLT Decoder - Synthetic Suite (tileGeoJson + decodeTile)", () => {
  it("finds 1150+ synthetic test fixtures across 0x01, 0x01-rust, and 0x02", () => {
    assert.ok(
      ALL_CASES.length >= 1150,
      `expected >= 1150 fixtures, found ${ALL_CASES.length}`,
    );
  });

  for (const tc of ALL_CASES) {
    it(`tileGeoJson: ${tc.name}`, () => {
      const mltBytes = new Uint8Array(readFileSync(tc.mltPath));
      const expected = JSON.parse(readFileSync(tc.jsonPath, "utf-8"));
      const actual = tileGeoJson(mltBytes);
      valuesEqual(actual, expected);
    });

    const shortV2 = tc.name.startsWith("0x02/")
      ? tc.name.slice("0x02/".length)
      : null;
    const isVtGap =
      shortV2 !== null && V2_VT_GAPS.some((re) => re.test(shortV2));

    if (!isVtGap) {
      it(`decodeTile: ${tc.name}`, () => {
        const mltBytes = new Uint8Array(readFileSync(tc.mltPath));
        const expected = JSON.parse(readFileSync(tc.jsonPath, "utf-8"));
        const actual = vectorTileToFeatureCollection(decodeTile(mltBytes));
        valuesEqual(actual, expected);
      });
    }
  }
});
