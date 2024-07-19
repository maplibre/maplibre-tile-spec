package com.mlt.converter.encodings.fsst;

public record SymbolTable(byte[] symbols, int[] symbolLengths, byte[] compressedData) {
  @Override
  public String toString() {
    return "SymbolTable{weight="
        + weight()
        + ", symbols=byte["
        + symbols.length
        + "], symbolLengths=int["
        + symbolLengths.length
        + "], compressedData=byte["
        + compressedData.length
        + "]}";
  }

  public int weight() {
    return symbols.length + symbolLengths.length + compressedData.length;
  }
}
