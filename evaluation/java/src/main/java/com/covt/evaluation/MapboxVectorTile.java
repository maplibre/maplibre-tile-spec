package com.covt.evaluation;

import java.util.List;

public record MapboxVectorTile(List<Layer> layers, int gzipCompressedMvtSize, int mvtSize){
}
