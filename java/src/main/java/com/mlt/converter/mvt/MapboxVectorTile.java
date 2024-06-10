package com.mlt.converter.mvt;

import java.util.List;
import com.mlt.data.Layer;

public record MapboxVectorTile(List<Layer> layers){
}
