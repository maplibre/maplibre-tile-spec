package com.mlt.converter.encodings.fsst;

public record SymbolTable(byte[] symbols, int[] symbolLengths, byte[] compressedData) {}
