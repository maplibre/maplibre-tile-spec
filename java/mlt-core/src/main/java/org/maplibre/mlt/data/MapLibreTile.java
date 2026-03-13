package org.maplibre.mlt.data;

import java.util.Collection;
import java.util.stream.Stream;
import org.jetbrains.annotations.NotNull;

public class MapLibreTile implements LayerProvider {
  @NotNull private final Collection<Layer> layers;

  public MapLibreTile(@NotNull Collection<Layer> layers) {
    this.layers = layers;
  }

  @Override
  public @NotNull Stream<Layer> getLayerStream(boolean parallel) {
    return parallel ? layers.parallelStream() : layers.stream();
  }
}
