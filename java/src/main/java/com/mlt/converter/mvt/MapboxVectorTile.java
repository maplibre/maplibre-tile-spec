package com.mlt.converter.mvt;

import com.mlt.data.Layer;
import java.util.List;

public record MapboxVectorTile(List<Layer> layers) {}
