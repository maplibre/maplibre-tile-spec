package org.maplibre.mlt.data;

import java.util.List;

public record Layer(String name, List<Feature> features, int tileExtent) {}
