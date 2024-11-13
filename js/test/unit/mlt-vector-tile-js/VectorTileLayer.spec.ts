import * as fs from 'fs';

import * as vt from '../../../src/mlt-vector-tile-js/index';
import type { VectorTileLayer as MvtVectorTileLayer } from '@mapbox/vector-tile';
import loadTile from './LoadMLTTile';

describe("VectorTileLayer", () => {
  it("should be assignable to MVT VectorTileLayer", () => {
    const tile: vt.VectorTile = loadTile('../test/expected/bing/4-13-6.mlt', '../test/expected/bing/4-13-6.mlt.meta.pbf');

    const layer: vt.VectorTileLayer = tile.layers['road'];
    const mvtLayer: MvtVectorTileLayer = layer;
  });

  it("should have the correct number of features", () => {
    const tile: vt.VectorTile = loadTile('../test/expected/bing/4-13-6.mlt', '../test/expected/bing/4-13-6.mlt.meta.pbf');

    expect(tile.layers['water_feature'].length).toEqual(20);
    expect(tile.layers['road'].length).toEqual(18);
    expect(tile.layers['land_cover_grass'].length).toEqual(1);
    expect(tile.layers['country_region'].length).toEqual(6);
    expect(tile.layers['land_cover_forest'].length).toEqual(1);
    expect(tile.layers['road_hd'].length).toEqual(2);
    expect(tile.layers['vector_background'].length).toEqual(1);
    expect(tile.layers['populated_place'].length).toEqual(28);
    expect(tile.layers['admin_division1'].length).toEqual(10);
  });
});
