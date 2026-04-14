package org.maplibre.mlt.data;

import java.util.SequencedCollection;
import org.jetbrains.annotations.NotNull;

public record Layer(
    @NotNull String name, @NotNull SequencedCollection<Feature> features, int tileExtent) {}
