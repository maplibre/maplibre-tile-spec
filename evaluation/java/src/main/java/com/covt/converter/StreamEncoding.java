package com.covt.converter;

public enum StreamEncoding {
    PLAIN,
    VARINT,
    VARINT_ZIG_ZAG,
    /* Without ZigZag encoding so only positive integers */
    VARINT_DELTA,
    VARINT_DELTA_ZIG_ZAG,
    RLE,
    BOOLEAN_RLE,
    BYTE_RLE,
    /* Without ZigZag encoding so only positive integers */
    FAST_PFOR_DELTA,
    FAST_PFOR_DELTA_ZIG_ZAG
}
