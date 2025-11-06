package org.maplibre.mlt.converter.encodings.fsst;

import nl.bartlouwers.fsst.*;

class FsstJni implements Fsst {
  private static nl.bartlouwers.fsst.Fsst impl = null;

  static {
    try {
      var impl = new FsstImpl();
      impl.encode(new byte[] {});
      FsstJni.impl = impl;
    } catch (UnsatisfiedLinkError e) {
      System.err.println("Failed to load native FSST: " + e.getMessage());
    }
  }

  public SymbolTable encode(byte[] data) {
    return impl.encode(data);
  }

  public byte[] decode(SymbolTable table) {
    return impl.decode(table);
  }

  public byte[] decode(
      byte[] symbols, int[] symbolLengths, byte[] compressedData, int decompressedLength) {
    return impl.decode(symbols, symbolLengths, compressedData, decompressedLength);
  }

  public static boolean isLoaded() {
    return (impl != null);
  }
}
