package org.maplibre.mlt.ffi;

import java.io.IOException;
import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.util.Locale;

/**
 * Loads the {@code mlt_ffi} native library once per class loader.
 * The library comes from the {@value #LIBRARY_PROPERTY} system property if set, else from the jar resources.
 */
public final class NativeLibrary {

  /** System property naming a library file that is loaded instead of the packaged one. */
  public static final String LIBRARY_PROPERTY = "mlt.ffi.library";

  private static final String RESOURCE_ROOT = "/org/maplibre/mlt/ffi/native/";

  private static Path loadedFrom;

  private NativeLibrary() {}

  /** Loads the native library unless it is loaded already, throwing {@link UnsatisfiedLinkError} if that fails. */
  public static synchronized void load() {
    if (loadedFrom != null) {
      return;
    }
    String override = System.getProperty(LIBRARY_PROPERTY);
    Path path = override != null ? Path.of(override) : extract();
    try {
      System.load(path.toAbsolutePath().toString());
    } catch (UnsatisfiedLinkError e) {
      throw new UnsatisfiedLinkError("Cannot load the MLT native library " + path + ": " + e.getMessage());
    }
    loadedFrom = path;
  }

  /** Returns the file the library was loaded from, or null if it is not loaded yet. */
  static synchronized Path loadedFrom() {
    return loadedFrom;
  }

  /** Returns the resource directory the packaged library of this platform lives in. */
  static String resourcePrefix(String osName, String archName, boolean musl) {
    String os = osName.toLowerCase(Locale.ROOT);
    String arch = archName.toLowerCase(Locale.ROOT);
    boolean x86 = arch.equals("amd64") || arch.equals("x86_64");
    boolean arm = arch.equals("aarch64") || arch.equals("arm64");
    if (os.contains("linux") && x86) {
      return musl ? "linux-x86-64-musl" : "linux-x86-64";
    } else if (os.contains("linux") && arm && !musl) {
      return "linux-aarch64";
    } else if (os.contains("mac") && arm) {
      return "darwin-aarch64";
    } else if (os.contains("windows") && x86) {
      return "win32-x86-64";
    }
    throw new UnsatisfiedLinkError("No MLT native library is packaged for " + osName + " " + archName
      + (musl ? " (musl)" : "") + ", set -D" + LIBRARY_PROPERTY + " to one");
  }

  static String fileName(String osName) {
    String os = osName.toLowerCase(Locale.ROOT);
    if (os.contains("windows")) {
      return "mlt_ffi.dll";
    }
    return os.contains("mac") ? "libmlt_ffi.dylib" : "libmlt_ffi.so";
  }

  private static boolean isMusl(String osName, String archName) {
    if (!osName.toLowerCase(Locale.ROOT).contains("linux")) {
      return false;
    }
    boolean arm = archName.equals("aarch64") || archName.equals("arm64");
    String musl = arm ? "/lib/ld-musl-aarch64.so.1" : "/lib/ld-musl-x86_64.so.1";
    String glibc = arm ? "/lib/ld-linux-aarch64.so.1" : "/lib64/ld-linux-x86-64.so.2";
    return Files.exists(Path.of(musl)) && !Files.exists(Path.of(glibc));
  }

  private static Path extract() {
    String osName = System.getProperty("os.name");
    String archName = System.getProperty("os.arch");
    String name = fileName(osName);
    String resource = RESOURCE_ROOT + resourcePrefix(osName, archName, isMusl(osName, archName)) + "/" + name;
    try (InputStream in = NativeLibrary.class.getResourceAsStream(resource)) {
      if (in == null) {
        throw new UnsatisfiedLinkError("The MLT native library resource " + resource + " is missing from the jar");
      }
      Path dir = Files.createTempDirectory("mlt-ffi");
      Path file = dir.resolve(name);
      Files.copy(in, file, StandardCopyOption.REPLACE_EXISTING);
      file.toFile().deleteOnExit();
      dir.toFile().deleteOnExit();
      return file;
    } catch (IOException e) {
      throw new UnsatisfiedLinkError("Cannot extract the MLT native library " + resource + ": " + e.getMessage());
    }
  }
}
