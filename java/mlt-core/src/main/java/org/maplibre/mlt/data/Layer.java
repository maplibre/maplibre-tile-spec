package org.maplibre.mlt.data;

import java.util.Objects;
import java.util.SequencedCollection;
import org.jetbrains.annotations.NotNull;

public record Layer(
    @NotNull String name, @NotNull SequencedCollection<Feature> features, int tileExtent) {
  public Layer {
    Objects.requireNonNull(name);
    Objects.requireNonNull(features);
    if (name.isEmpty()) {
      throw new IllegalArgumentException("Missing layer name");
    }
  }
}
