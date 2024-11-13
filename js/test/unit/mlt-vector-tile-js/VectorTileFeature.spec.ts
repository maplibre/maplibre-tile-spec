import * as fs from 'fs';

import { MltDecoder, TileSetMetadata } from '../../../src/index';
import * as vt from '../../../src/mlt-vector-tile-js/index';
import type { VectorTileFeature as MvtVectorTileFeature } from '@mapbox/vector-tile';
import loadTile from './LoadMLTTile';
import { default as loadMvtTile } from './LoadMVTTile';

describe("VectorTileFeature", () => {
  it("should be assignable to MVT VectorTileFeature", () => {
    const tile: vt.VectorTile = loadTile('../test/expected/bing/4-13-6.mlt', '../test/expected/bing/4-13-6.mlt.meta.pbf');
    const feature : vt.VectorTileFeature = tile.layers['road'].feature(0);
    const mvtFeature : MvtVectorTileFeature = feature;
  });

  it("should return valid GeoJSON for a feature", () => {
    const tile: vt.VectorTile = loadTile('../test/expected/bing/4-13-6.mlt', '../test/expected/bing/4-13-6.mlt.meta.pbf');

    const feature = tile.layers['road'].feature(0);
    const geojsonFeature: any = feature.toGeoJSON(13, 6, 4);
    const geometry: any = geojsonFeature.geometry;
    expect(geometry.coordinates[0][0][0]).not.toBe(undefined);
  });

  it("should load geometry", () => {
    const tile: vt.VectorTile = loadTile('../test/expected/bing/4-13-6.mlt', '../test/expected/bing/4-13-6.mlt.meta.pbf');
    const feature = tile.layers['road'].feature(0);
    expect(feature.loadGeometry()[0][0].x).not.toBe(undefined);
  });

  it("should have a valid extent", () => {
    const tile: vt.VectorTile = loadTile('../test/expected/bing/4-13-6.mlt', '../test/expected/bing/4-13-6.mlt.meta.pbf');
    expect(tile.layers['road'].feature(0).extent).not.toBe(undefined);
  });

  it("should have same loadGeometry output as MVT", () => {
    const tile: vt.VectorTile = loadTile('../test/expected/bing/4-13-6.mlt', '../test/expected/bing/4-13-6.mlt.meta.pbf');

    const mvtTile = loadMvtTile('../test/fixtures/bing/4-13-6.mvt');

    for (let i = 0; i < tile.layers['country_region'].length; i++) {
      const mltFeature = tile.layers['country_region'].feature(i);
      const mvtFeature = mvtTile.layers['country_region'].feature(i);
      expect(mltFeature.loadGeometry()).toEqual(mvtFeature.loadGeometry());
    }
  });

  it.skip("should have same type output as MVT", () => {
    const tile: vt.VectorTile = loadTile('../test/expected/bing/4-13-6.mlt', '../test/expected/bing/4-13-6.mlt.meta.pbf');

    const mvtTile = loadMvtTile('../test/fixtures/bing/4-13-6.mvt');

    for (let i = 0; i < tile.layers['land_cover_grass'].length; i++) {
      const mltFeature = tile.layers['land_cover_grass'].feature(i);
      const mvtFeature = mvtTile.layers['land_cover_grass'].feature(i);
      expect(mltFeature.type).toEqual(mvtFeature.type);
    }
  });
});
