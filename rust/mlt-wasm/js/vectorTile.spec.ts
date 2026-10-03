import { readFile } from "node:fs/promises";
import type Point from "@mapbox/point-geometry";
import { describe, expect, it } from "vitest";
import { decodeTile, type MltFeature, type MltLayer } from "./vectorTile";

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
