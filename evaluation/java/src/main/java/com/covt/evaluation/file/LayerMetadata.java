package com.covt.evaluation.file;

public record LayerMetadata(String layerName, int extent, int numFeatures, int numColumns, ColumnMetadata[] columnMetadata) { }
