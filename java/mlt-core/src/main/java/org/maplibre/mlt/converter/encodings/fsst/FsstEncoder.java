package org.maplibre.mlt.converter.encodings.fsst;

public class FsstEncoder {
  public static Fsst INSTANCE;

  public static boolean useNative(boolean value) {
    if (value) {
      if (!FsstJni.isLoaded()) {
        INSTANCE = new FsstJni();
        return true;
      } else {
        return false;
      }
    } else {
      INSTANCE = new FsstJava();
      return true;
    }
  }

  private static Fsst getInstance() {
    if (INSTANCE == null) {
      useNative(false);
    }
    return INSTANCE;
  }

  private FsstEncoder() {}

  public static SymbolTable encode(byte[] data) {
    return getInstance().encode(data);
  }

  public static byte[] decode(
      byte[] symbols, int[] symbolLengths, byte[] compressedData, int decompressedLength) {
    return getInstance().decode(symbols, symbolLengths, compressedData, decompressedLength);
  }

  /**
   * @deprecated use {@link #decode(byte[], int[], byte[], int)} instead with an explicit length
   */
  @Deprecated
  public static byte[] decode(byte[] symbols, int[] symbolLengths, byte[] compressedData) {
    return getInstance().decode(symbols, symbolLengths, compressedData);
  }
}
