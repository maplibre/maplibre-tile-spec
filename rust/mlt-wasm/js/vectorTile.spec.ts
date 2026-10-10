import { readFile } from "node:fs/promises";
import type Point from "@mapbox/point-geometry";
import { describe, expect, it } from "vitest";
import { decodeTileColumns } from "./columns";
import {
  decodeTile,
  decodeTile3D,
  type MltFeature,
  MltLayer,
} from "./vectorTile";

interface Fixture {
  feature: MltFeature;
  /** The fixture's GeoJSON rings, which repeat their first position at the end. */
  polygons: number[][][][];
}

const fixtures = new Map<string, Promise<Fixture>>();

/** Reads and decodes each fixture once, however many tests use it. */
function load(name: string): Promise<Fixture> {
  let fixture = fixtures.get(name);
  if (!fixture) {
    fixture = readFixture(name);
    fixtures.set(name, fixture);
  }
  return fixture;
}

async function readFixture(name: string): Promise<Fixture> {
  const path = (ext: string) =>
    new URL(`../../../test/synthetic/0x01/${name}.${ext}`, import.meta.url);
  const [mlt, json] = await Promise.all([
    readFile(path("mlt")),
    readFile(path("json"), "utf8"),
  ]);
  const tile = decodeTile(new Uint8Array(mlt));
  const { geometry } = JSON.parse(json).features[0];
  return {
    feature: (Object.values(tile.layers)[0] as MltLayer).feature(0),
    polygons:
      geometry.type === "Polygon"
        ? [geometry.coordinates]
        : geometry.coordinates,
  };
}

const xy = (ring: Point[]) => ring.map((p) => [p.x, p.y]);

// @maplibre/vt-pbf writes `ring.length - 1` points and a ClosePath, so rings must repeat their first point.
describe("polygon rings are closed", () => {
  for (const name of ["poly_hole", "poly_multi"]) {
    it(`loadGeometry on ${name}`, async () => {
      const { feature, polygons } = await load(name);
      expect(feature.loadGeometry().map(xy)).toEqual(polygons.flat());
    });

    it(`loadPolygons on ${name}`, async () => {
      const { feature, polygons } = await load(name);
      expect(feature.loadPolygons().map((polygon) => polygon.map(xy))).toEqual(
        polygons,
      );
    });
  }

  it("the closing point is a separate object from the first", async () => {
    const [ring] = (await load("poly")).feature.loadGeometry();
    expect(ring).toHaveLength(4);
    expect(ring[3]).not.toBe(ring[0]);
  });
});

describe("malformed offsets", () => {
  it("are rejected, not read past", async () => {
    const mlt = await readFile(
      new URL("../../../test/synthetic/0x01/poly_hole.mlt", import.meta.url),
    );
    const [layer] = decodeTileColumns(new Uint8Array(mlt)).layers;
    const { ringOffsets } = layer.geometry;
    if (ringOffsets === undefined)
      throw new Error("poly_hole has no ring offsets");
    const backwards = Uint32Array.from(ringOffsets);
    [backwards[1], backwards[2]] = [backwards[2], backwards[1]];
    const broken = new MltLayer({
      ...layer,
      geometry: { ...layer.geometry, ringOffsets: backwards },
    });
    expect(() => broken.feature(0).loadGeometry()).toThrow(
      /ends before it starts/,
    );
  });
});

describe("layers keyed by name", () => {
  it("rejects a tile whose layers share a name, rather than keeping one", async () => {
    // A tile is a sequence of layer frames, so a fixture twice over is a tile of two layers.
    const point = new Uint8Array(
      await readFile(
        new URL("../../../test/synthetic/0x01/point.mlt", import.meta.url),
      ),
    );
    const twice = new Uint8Array(point.length * 2);
    twice.set(point);
    twice.set(point, point.length);
    expect(Object.keys(decodeTile(point).layers)).toHaveLength(1);
    expect(() => decodeTile(twice)).toThrow(/two layers named/);
  });
});

describe("feature indexes", () => {
  it("rejects an index that is not a feature, naming the layer", async () => {
    const read = async (name: string) =>
      new Uint8Array(
        await readFile(
          new URL(`../../../test/synthetic/${name}.mlt`, import.meta.url),
        ),
      );
    const [flat] = Object.values(decodeTile(await read("0x01/point")).layers);
    const [raised] = Object.values(
      decodeTile3D(await read("0x02/z_point")).layers,
    );
    for (const layer of [flat, raised]) {
      expect(layer.length).toBe(1);
      expect(() => layer.feature(0)).not.toThrow();
      for (const index of [-1, 1, 0.5, Number.NaN]) {
        expect(() => layer.feature(index)).toThrow(
          new RangeError(`layer "${layer.name}" has no feature ${index}`),
        );
      }
    }
  });
});
