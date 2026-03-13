package org.maplibre.mlt.data;

import java.util.SequencedCollection;
import java.util.stream.Stream;
import org.apache.commons.lang3.tuple.Triple;
import org.jetbrains.annotations.NotNull;

public class MapboxVectorTile implements LayerProvider {
  private @NotNull SequencedCollection<Layer> layers;
  private Triple<Integer, Integer, Integer> tileId;

  public MapboxVectorTile(@NotNull SequencedCollection<Layer> layers) {
    this.layers = layers;
  }

  public MapboxVectorTile(
      @NotNull SequencedCollection<Layer> layers,
      @NotNull Triple<Integer, Integer, Integer> tileId) {
    this(layers);
    this.tileId = tileId;
  }

  public void setTileId(@NotNull Triple<Integer, Integer, Integer> tileId) {
    this.tileId = tileId;
  }

  public @NotNull Triple<Integer, Integer, Integer> tileId() {
    return tileId;
  }

  @Override
  public long getLayerCount() {
    return layers.size();
  }

  @Override
  public @NotNull Stream<Layer> getLayerStream(boolean parallel) {
    return parallel ? layers.parallelStream() : layers.stream();
  }
}
