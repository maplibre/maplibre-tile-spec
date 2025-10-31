package org.maplibre.mlt.converter.encodings.fsst;

import java.nio.file.FileSystems;

class FsstJni implements Fsst {
  private static boolean isLoaded = false;

  static {
    final String os = System.getProperty("os.name").toLowerCase();
    final boolean isWindows = os.contains("win");
    String moduleDir = "../build/FsstWrapper.so";
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
      System.out.println("Error: " + e.getMessage() + " - " + modulePath);
    }
  }

  public SymbolTable encode(byte[] data) {
    return FsstJni.compress(data);
  }

  public static boolean isLoaded() { return isLoaded; }

  public static native SymbolTable compress(byte[] inputBytes);
}
