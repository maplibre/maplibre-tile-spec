package com.covt.converter;

public enum ColumnEncoding {
    PLAIN,
    VARINT,
    DELTA_VARINT,
    RLE,
    BOOLEAN_RLE,
    BYTE_RLE,
    DICTIONARY,
    LOCALIZED_DICTIONARY,
    ORDERED_GEOMETRY_ENCODING,
    INDEXED_COORDINATE_ENCODING,
}
