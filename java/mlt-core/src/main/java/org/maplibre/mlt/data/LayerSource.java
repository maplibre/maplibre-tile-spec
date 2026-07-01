package org.maplibre.mlt.data;

import java.util.stream.Stream;
import org.jetbrains.annotations.NotNull;

public interface LayerSource {
  default long getLayerCount() {
    return getLayerStream().count();
  }

  @NotNull
  default Iterable<Layer> getLayers() {
    return () -> getLayerStream().iterator();
  }

  @NotNull
  default Stream<Layer> getLayerStream() {
    return getLayerStream(false);
  }

  @NotNull
  Stream<Layer> getLayerStream(boolean parallel);
}
