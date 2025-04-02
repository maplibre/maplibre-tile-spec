package com.mlt.converter.mvt;

import com.mlt.data.Layer;
import java.util.List;
import org.apache.commons.lang3.tuple.Triple;

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
