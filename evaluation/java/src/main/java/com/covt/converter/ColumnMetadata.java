package com.covt.converter;

import java.util.TreeMap;

public record ColumnMetadata(ColumnDataType columnDataType, ColumnType columnType,
                             TreeMap<StreamType, StreamMetadata> streams){
}
