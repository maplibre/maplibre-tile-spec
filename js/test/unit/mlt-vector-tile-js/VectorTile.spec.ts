import * as fs from 'fs';

import type { VectorTile as MvtVectorTile } from '@mapbox/vector-tile';
import * as vt from '../../../src/mlt-vector-tile-js/index';
import loadTile from './LoadMLTTile';

describe("VectorTile", () => {
  it("should have all layers", () => {
    const tile: vt.VectorTile = loadTile('../test/expected/bing/4-13-6.mlt', '../test/expected/bing/4-13-6.mlt.meta.pbf');

    expect(Object.keys(tile.layers)).toEqual([
      "water_feature",
      "road",
      "land_cover_grass",
      "country_region",
      "land_cover_forest",
      "road_hd",
      "vector_background",
      "populated_place",
      "admin_division1",
    ]);
  })

  it("should return valid GeoJSON for a feature", () => {
    const tile: vt.VectorTile = loadTile('../test/expected/bing/4-13-6.mlt', '../test/expected/bing/4-13-6.mlt.meta.pbf');

    const geojson = tile.layers['road'].feature(0).toGeoJSON(13, 6, 4);

    expect(geojson.type).toEqual('Feature');
    expect(geojson.geometry.type).toEqual('MultiLineString');
  })

  it("should be equivalent to MVT VectorTile", () => {
    const tile: vt.VectorTile = loadTile('../test/expected/bing/4-13-6.mlt', '../test/expected/bing/4-13-6.mlt.meta.pbf');

    const mvtType : MvtVectorTile = tile;
  });
});
