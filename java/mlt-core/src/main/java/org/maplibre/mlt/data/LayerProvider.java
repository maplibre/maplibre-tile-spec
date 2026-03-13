package org.maplibre.mlt.data;

import jakarta.annotation.Nullable;
import java.util.stream.Stream;
import org.jetbrains.annotations.NotNull;

public interface LayerProvider {
  default long getLayerCount() {
    return getLayerStream().count();
  }

  @NotNull
  default Iterable<String> getLayerNames() {
    return () -> getLayerStream().map(Layer::name).iterator();
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

  @Nullable
  default Layer getLayer(@NotNull String name) {
    return getLayerStream().filter(layer -> layer.name().equals(name)).findFirst().orElse(null);
  }
}
