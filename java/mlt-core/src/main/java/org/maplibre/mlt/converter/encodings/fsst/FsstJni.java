package org.maplibre.mlt.converter.encodings.fsst;

import nl.bartlouwers.fsst.*;

public class FsstJni implements Fsst {
  private static nl.bartlouwers.fsst.Fsst impl = null;
  private static Error loadError;

  static {
    try {
      var impl = new FsstImpl();
      impl.encode(new byte[] {});
      FsstJni.impl = impl;
    } catch (UnsatisfiedLinkError | ExceptionInInitializerError e) {
      loadError = e;
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

  public static Error getLoadError() {
    return loadError;
  }
}
