package com.mlt.converter.mvt;

import java.util.List;

public record Layer(String name, List<Feature> features, int tileExtent) { }
