package com.covt.converter;

import java.util.LinkedHashMap;

public record ColumnMetadata(ColumnDataType columnDataType, ColumnEncoding columnEncoding,
                             LinkedHashMap<String, StreamMetadata> streams){
}


/*
*
* public class ColumnMetadata{


    public ColumnMetadata(ColumnDataType columnDataType, ColumnEncoding columnEncoding,
                             LinkedHashMap<String, StreamMetadata> streams){

    }
}
* */
