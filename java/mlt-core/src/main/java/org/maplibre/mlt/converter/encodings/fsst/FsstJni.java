package org.maplibre.mlt.converter.encodings.fsst;

import java.nio.file.FileSystems;

public class FsstJni implements Fsst {

  private static boolean isLoaded = false;
  private static UnsatisfiedLinkError loadError;

  static {
    String os = System.getProperty("os.name").toLowerCase();
    boolean isWindows = os.contains("win");
    String moduleDir = "build/FsstWrapper.so";
    if (isWindows) {
      // TODO: figure out how to get cmake to put in common directory
      moduleDir = "build/Release/FsstWrapper.so";
    }
    String modulePath =
        FileSystems.getDefault().getPath(moduleDir).normalize().toAbsolutePath().toString();
    try {
      System.load(modulePath);
      isLoaded = true;
    } catch (UnsatisfiedLinkError e) {
      loadError = e;
    }
  }

  public SymbolTable encode(byte[] data) {
    return FsstJni.compress(data);
  }

  public static boolean isLoaded() {
    return isLoaded;
  }

  public static UnsatisfiedLinkError getLoadError() {
    return loadError;
  }

  public static native SymbolTable compress(byte[] inputBytes);
}
