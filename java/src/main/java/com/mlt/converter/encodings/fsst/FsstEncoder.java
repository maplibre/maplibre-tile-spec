package com.mlt.converter.encodings.fsst;

import java.io.IOException;

public interface FsstEncoder {
  Fsst INSTANCE = new FsstJni();

  static SymbolTable encode(byte[] data) {
    return INSTANCE.encode(data);
  }

  static byte[] decode(byte[] symbols, int[] symbolLengths, byte[] compressedData)
      throws IOException {
    return INSTANCE.decode(symbols, symbolLengths, compressedData);
  }
}
