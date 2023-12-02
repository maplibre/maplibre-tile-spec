package com.covt.decoder;


import com.covt.converter.ColumnMetadata;
import java.util.LinkedHashMap;

public record LayerMetadata(String layerName, int extent, int numFeatures, int numColumns, LinkedHashMap<String, ColumnMetadata> columnMetadata) {
}
