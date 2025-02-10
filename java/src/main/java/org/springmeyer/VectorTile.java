package org.springmeyer;

import java.util.Map;

/*
 * This is a java port of https://github.com/mapbox/vector-tile-js
 */
public class VectorTile {
  public Map<String, VectorTileLayer> layers;

  public VectorTile(Pbf pbf, int end) throws IllegalArgumentException {
    this.layers = pbf.readFields(VectorTile::readTile, new java.util.LinkedHashMap<>(), end);
  }

  private static void readTile(int tag, Map<String, VectorTileLayer> layers, Pbf pbf)
      throws IllegalArgumentException {
    if (tag == 3) {
      VectorTileLayer layer = new VectorTileLayer(pbf, pbf.readVarint() + pbf.pos);
      if (layer.length > 0) {
        layers.put(layer.name, layer);
      }
    }
  }
}
