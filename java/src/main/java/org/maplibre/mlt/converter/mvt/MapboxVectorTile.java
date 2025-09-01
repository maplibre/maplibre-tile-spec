package org.maplibre.mlt.converter.mvt;

import java.util.List;
import org.apache.commons.lang3.tuple.Triple;
import org.maplibre.mlt.data.Layer;

public class MapboxVectorTile {
  private List<Layer> layers;
  private Triple<Integer, Integer, Integer> tileId;

  public MapboxVectorTile(List<Layer> layers) {
    this.layers = layers;
  }

  public MapboxVectorTile(List<Layer> layers, Triple<Integer, Integer, Integer> tileId) {
    this(layers);
    this.tileId = tileId;
  }

  public void setTileId(Triple<Integer, Integer, Integer> tileId) {
    this.tileId = tileId;
  }

  public List<Layer> layers() {
    return layers;
  }

  public Triple<Integer, Integer, Integer> tileId() {
    return tileId;
  }
}
