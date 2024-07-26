package com.mlt.converter.encodings.fsst;

import java.io.ByteArrayOutputStream;
import java.io.IOException;

public interface Fsst {
  SymbolTable encode(byte[] data);

  default byte[] decode(SymbolTable encoded) {
    return decode(encoded.symbols(), encoded.symbolLengths(), encoded.compressedData());
  }

  default byte[] decode(byte[] symbols, int[] symbolLengths, byte[] compressedData) {
    ByteArrayOutputStream decodedData = new ByteArrayOutputStream();

    int[] symbolOffsets = new int[symbolLengths.length];
    for (int i = 1; i < symbolLengths.length; i++) {
      symbolOffsets[i] = symbolOffsets[i - 1] + symbolLengths[i - 1];
    }

    for (int i = 0; i < compressedData.length; i++) {
      // In Java a byte[] is signed [-128 to 127], whereas in C++ it is unsigned [0 to 255]
      // So we do a bit shifting operation to convert the values into unsigned values for easier
      // handling
      int symbolIndex = compressedData[i] & 0xFF;
      // 255 is our escape byte -> take the next symbol as it is
      if (symbolIndex == 255) {
        decodedData.write(compressedData[++i] & 0xFF);
      } else if (symbolIndex < symbolLengths.length) {
        decodedData.write(symbols, symbolOffsets[symbolIndex], symbolLengths[symbolIndex]);
      }
    }

    return decodedData.toByteArray();
  }
}
