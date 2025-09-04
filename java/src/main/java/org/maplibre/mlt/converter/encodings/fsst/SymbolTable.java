package org.maplibre.mlt.converter.encodings.fsst;

import java.util.Arrays;

/**
 * Output of FSST-encoding: a symbol table, and text encoded with that symbol table.
 *
 * @param symbols Up to 255 symbols in the table concatenated together
 * @param symbolLengths Lengths of each of those 255 symbols in the table
 * @param compressedData Encoded text where a value less than 255 refers to that symbol from the
 *     table, and a value of 255 means the next byte should be used directly.
 */
public record SymbolTable(
    byte[] symbols, int[] symbolLengths, byte[] compressedData, int decompressedLength) {
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
        + "], decompressedLength="
        + decompressedLength
        + '}';
  }

  public int weight() {
    return symbols.length + symbolLengths.length + compressedData.length;
  }

  @Override
  public boolean equals(Object o) {
    return this == o
        || (o instanceof SymbolTable that
            && Arrays.equals(symbols, that.symbols)
            && Arrays.equals(symbolLengths, that.symbolLengths)
            && Arrays.equals(compressedData, that.compressedData));
  }

  @Override
  public int hashCode() {
    int result = Arrays.hashCode(symbols);
    result = 31 * result + Arrays.hashCode(symbolLengths);
    result = 31 * result + Arrays.hashCode(compressedData);
    return result;
  }
}
