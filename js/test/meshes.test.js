import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";
import {
  MltGeometryType,
  columnValue,
  decodeTile,
  decodeTile3D,
  decodeTileColumns,
  featureGeometry,
  geometryStarts,
  isMeshes,
  isTrianglesOnly,
  mValueSlice,
  tileGeoJson,
  toElevation,
} from "../src/index.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const MESHES_DIR = path.resolve(__dirname, "../../test/fixtures/meshes/tiles");
const OSM_XLARGE = path.resolve(__dirname, "../../test/fixtures/osm/xlarge.mlt");
const SYNTHETIC_DIR = path.resolve(__dirname, "../../test/synthetic");
const VIEWER_HTML = path.resolve(__dirname, "../viewer/meshes.html");

function triples(words) {
  const out = [];
  for (let i = 0; i < words.length; i += 3) {
    out.push([words[i], words[i + 1], words[i + 2]]);
  }
  return out;
}

describe("Pure JS MLT Decoder - 3D CityGML Meshes & Columnar API", () => {
  const meshTiles = readdirSync(MESHES_DIR, { recursive: true })
    .map(String)
    .filter((p) => p.endsWith(".mlt"))
    .map((p) => p.replaceAll("\\", "/"))
    .sort();

  it("finds the v2 CityGML mesh fixtures", () => {
    assert.deepEqual(meshTiles, ["15/17294/10590.mlt", "16/34588/21180.mlt"]);
  });

  for (const rel of meshTiles) {
    it(`decodes 3D mesh tile ${rel} across decodeTileColumns, featureGeometry, decodeTile3D, and tileGeoJson`, () => {
      const data = new Uint8Array(
        readFileSync(path.join(MESHES_DIR, rel)),
      );
      const geo = tileGeoJson(data);
      const { layers } = decodeTileColumns(data);
      assert.equal(layers.length, 1);

      const [layer] = layers;
      assert.equal(layer.name, "buildings");
      assert.equal(layer.version, 2);
      assert.equal(layer.featureCount, geo.features.length);
      assert.equal(layer.geometry.dimension, 3);
      assert.equal(layer.geometry.zStep, -2);
      assert.equal(layer.geometry.zDatum, "ground");
      assert.equal(layer.geometry.meshes, true);
      assert.equal(isMeshes(layer.geometry), true);
      assert.equal(isTrianglesOnly(layer.geometry), false);

      const starts = geometryStarts(layer.geometry);
      assert.equal(starts.featureGeometries.length, layer.featureCount + 1);

      const partCol = layer.mValues.find((c) => c.name === "part");
      assert.ok(partCol);
      assert.equal(partCol.scope, "triangle");
      assert.equal(partCol.type, "string");
      assert.equal(partCol.present, undefined);

      const tile2d = decodeTile(data).layers.buildings;
      assert.equal(tile2d.zStep, -2);
      assert.equal(tile2d.zDatum, "ground");
      assert.throws(() => tile2d.feature(0).loadPolygons(), /is a mesh/);

      const tile3d = decodeTile3D(data).layers.buildings;
      assert.equal(tile3d.zStep, -2);
      assert.equal(tile3d.zDatum, "ground");

      for (let f = 0; f < layer.featureCount; f++) {
        const expected = geo.features[f];
        const fg = featureGeometry(layer, f);
        assert.equal(fg.kind, "mesh");
        assert.deepEqual(triples(fg.vertices), expected.geometry.coordinates);
        assert.deepEqual(triples(fg.triangles), expected.triangles);

        const f3d = tile3d.feature(f);
        assert.equal(f3d.id, expected.id);
        assert.equal(f3d.zDatum, "ground");
        assert.equal(f3d.properties.gml_id, expected.properties.gml_id);
        assert.deepEqual(f3d.loadGeometry(), [expected.geometry.coordinates]);
        assert.deepEqual(triples(f3d.loadTriangles()), expected.triangles);
        assert.deepEqual(f3d.loadMValues("part"), expected.properties["m:part"]);
        assert.equal(f3d.loadMValues("missing"), undefined);
        assert.deepEqual(
          mValueSlice(layer, partCol, f),
          expected.properties["m:part"],
        );
      }
    });
  }

  it("supports layer name filtering in decodeTileColumns without decoding skipped layers", () => {
    const f1 = new Uint8Array(
      readFileSync(path.join(SYNTHETIC_DIR, "0x01/fpf_align_1.mlt")),
    );
    const f2 = new Uint8Array(
      readFileSync(path.join(SYNTHETIC_DIR, "0x02/z_point.mlt")),
    );
    const f3 = new Uint8Array(
      readFileSync(path.join(SYNTHETIC_DIR, "0x01/fpf_align_2.mlt")),
    );
    const combined = new Uint8Array(f1.length + f2.length + f3.length + f2.length);
    combined.set(f1, 0);
    combined.set(f2, f1.length);
    combined.set(f3, f1.length + f2.length);
    combined.set(f2, f1.length + f2.length + f3.length);

    assert.deepEqual(
      decodeTileColumns(combined).layers.map((l) => l.name),
      ["a", "layer1", "aa", "layer1"],
    );
    assert.deepEqual(
      decodeTileColumns(combined, { layers: ["aa", "layer1"] }).layers.map(
        (l) => l.name,
      ),
      ["layer1", "aa", "layer1"],
    );
    assert.deepEqual(decodeTileColumns(combined, { layers: [] }).layers, []);
    assert.deepEqual(
      decodeTileColumns(combined, { layers: ["missing"] }).layers,
      [],
    );
  });

  it("validates columnValue, mValueSlice, and toElevation edge cases", () => {
    const zMvals = new Uint8Array(
      readFileSync(path.join(SYNTHETIC_DIR, "0x02/z_mvalues.mlt")),
    );
    const [layer] = decodeTileColumns(zMvals).layers;
    const dist = layer.mValues.find((c) => c.name === "dist");
    assert.throws(() => columnValue(dist, 0), TypeError);
    assert.throws(() => mValueSlice(layer, dist, -1), RangeError);
    assert.throws(() => mValueSlice(layer, dist, layer.featureCount), RangeError);

    assert.equal(toElevation(1234, -2, "ground"), 12.34);
    assert.equal(Object.is(toElevation(-0, 0, "ground"), 0), true);
    assert.equal(toElevation(1_001_234, -2), 12.34);
    assert.equal(toElevation(10_012_345, -3), 12.345);
  });

  it("decodes large real-world OpenStreetMap tile (xlarge.mlt)", () => {
    const data = new Uint8Array(readFileSync(OSM_XLARGE));
    const { layers } = decodeTileColumns(data);
    assert.ok(layers.length > 0);
    let totalFeatures = 0;
    for (const layer of layers) {
      totalFeatures += layer.featureCount;
      if (layer.featureCount > 0 && !isTrianglesOnly(layer.geometry)) {
        const fg = featureGeometry(layer, 0);
        assert.ok(fg.vertices.length >= 0);
      }
    }
    assert.ok(totalFeatures > 1000, `expected > 1000 features, got ${totalFeatures}`);
  });

  it("viewer/meshes.html has valid importmap and module script targeting @maplibre/mlt-js", async () => {
    const html = readFileSync(VIEWER_HTML, "utf-8");
    const mapMatch = /<script type="importmap">\s*([\s\S]*?)\s*<\/script>/.exec(html);
    assert.ok(mapMatch, "missing importmap in viewer/meshes.html");
    const importMap = JSON.parse(mapMatch[1]);
    const relTarget = importMap.imports["@maplibre/mlt-js"];
    assert.equal(relTarget, "../min/columns.js");
    const resolvedEntry = path.resolve(path.dirname(VIEWER_HTML), relTarget);
    assert.ok(readFileSync(resolvedEntry, "utf-8").includes("decodeTileColumns"));

    const modMatch = /<script type="module">\s*([\s\S]*?)\s*<\/script>/.exec(html);
    assert.ok(modMatch, "missing module script in viewer/meshes.html");
    const syntaxCheck = spawnSync(
      process.execPath,
      ["--input-type=module", "--check"],
      { input: modMatch[1], encoding: "utf-8" },
    );
    assert.equal(syntaxCheck.status, 0, syntaxCheck.stderr);
    assert.ok(modMatch[1].includes('from "@maplibre/mlt-js"'));
    assert.ok(modMatch[1].includes("decodeTileColumns(data).layers.filter"));
    assert.ok(!modMatch[1].includes("await init()"));

    // Verify minified bundles decode identically to src/
    const minFull = await import("../min/index.js");
    const minCols = await import("../min/columns.js");
    const sample = new Uint8Array(
      readFileSync(path.join(MESHES_DIR, "16/34588/21180.mlt")),
    );
    assert.deepEqual(minCols.decodeTileColumns(sample), decodeTileColumns(sample));
    assert.deepEqual(minFull.tileGeoJson(sample), tileGeoJson(sample));
  });
});
