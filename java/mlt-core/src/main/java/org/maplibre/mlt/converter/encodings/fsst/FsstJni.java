package org.maplibre.mlt.converter.encodings.fsst;

import jakarta.annotation.Nullable;
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

  public @Nullable SymbolTable encode(byte[] data) {
    return (impl != null) ? impl.encode(data) : null;
  }

  public @Nullable byte[] decode(SymbolTable table) {
    return (impl != null) ? impl.decode(table) : null;
  }

  public @Nullable byte[] decode(
      byte[] symbols, int[] symbolLengths, byte[] compressedData, int decompressedLength) {
    return (impl != null)
        ? impl.decode(symbols, symbolLengths, compressedData, decompressedLength)
        : null;
  }

  public static boolean isLoaded() {
    return (impl != null);
  }

  public static Error getLoadError() {
    return loadError;
  }
}
