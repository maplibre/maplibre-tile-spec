package org.maplibre.mlt.ffi;

import java.io.File;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.attribute.PosixFilePermissions;
import java.util.Locale;
import java.util.UUID;

/**
 * Loads the {@code mlt_ffi} native library once per class loader.
 * The library comes from the {@value #LIBRARY_PROPERTY} system property if set, else from the jar resources.
 * <p>
 * A packaged library is extracted to a temporary file that is deleted on exit.
 * Windows cannot delete a library that is still loaded, so each load also deletes those of exited JVMs.
 */
public final class NativeLibrary {

  /** System property naming a library file that is loaded instead of the packaged one. */
  public static final String LIBRARY_PROPERTY = "mlt.ffi.library";

  private static final String RESOURCE_ROOT = "/org/maplibre/mlt/ffi/native/";

  static final String TEMP_PREFIX = "mlt-ffi-";

  static final String LOCK_SUFFIX = ".lck";

  private static Path loadedFrom;

  private NativeLibrary() {}

  /**
   * Loads the native library unless it is loaded already.
   *
   * @throws UnsatisfiedLinkError if the library is missing or cannot be loaded
   */
  public static synchronized void load() {
    if (loadedFrom != null) {
      return;
    }
    String override = System.getProperty(LIBRARY_PROPERTY);
    Path path = override != null ? Path.of(override) : extract();
    try {
      System.load(path.toAbsolutePath().toString());
    } catch (UnsatisfiedLinkError e) {
      throw linkError("Cannot load the MLT native library " + path + ": " + e.getMessage(), e);
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
    } else if (os.contains("linux") && arm) {
      return musl ? "linux-aarch64-musl" : "linux-aarch64";
    } else if (os.contains("mac") && arm) {
      return "darwin-aarch64";
    } else if (os.contains("mac") && x86) {
      return "darwin-x86-64";
    } else if (os.contains("freebsd") && x86) {
      return "freebsd-x86-64";
    } else if (os.contains("freebsd") && arm) {
      return "freebsd-aarch64";
    } else if (os.contains("windows") && x86) {
      return "win32-x86-64";
    } else if (os.contains("windows") && arm) {
      return "win32-aarch64";
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
    String arch = archName.toLowerCase(Locale.ROOT);
    boolean arm = arch.equals("aarch64") || arch.equals("arm64");
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
      Path tmp = Path.of(System.getProperty("java.io.tmpdir"));
      deleteStale(tmp.toFile());
      Path file = tmp.resolve(TEMP_PREFIX + UUID.randomUUID() + "-" + name);
      // like sqlite-jdbc: the lock exists first, so another JVM never deletes a library that is being extracted
      createPrivateFile(file.resolveSibling(file.getFileName() + LOCK_SUFFIX)).toFile().deleteOnExit();
      createPrivateFile(file).toFile().deleteOnExit();
      // writes into the existing file, as Files.copy would recreate it without the owner-only permissions
      try (OutputStream out = Files.newOutputStream(file)) {
        in.transferTo(out);
      }
      return file;
    } catch (IOException e) {
      throw linkError("Cannot extract the MLT native library " + resource + ": " + e.getMessage(), e);
    }
  }

  /**
   * Creates a new file only its owner can access, as other users of the shared temporary directory must not change it.
   * Fails if the path exists, so it never follows a planted link.
   */
  private static Path createPrivateFile(Path path) throws IOException {
    if (path.getFileSystem().supportedFileAttributeViews().contains("posix")) {
      return Files.createFile(path, PosixFilePermissions.asFileAttribute(PosixFilePermissions.fromString("rw-------")));
    }
    // the Windows temporary directory is private to the user already
    return Files.createFile(path);
  }

  /** Deletes the libraries in {@code tmp} that other JVMs extracted but could not delete on exit. */
  static void deleteStale(File tmp) {
    File[] stale = tmp.listFiles((dir, n) -> n.startsWith(TEMP_PREFIX) && !n.endsWith(LOCK_SUFFIX)
      && !new File(dir, n + LOCK_SUFFIX).exists());
    if (stale != null) {
      for (File file : stale) {
        file.delete();
      }
    }
  }

  private static UnsatisfiedLinkError linkError(String message, Throwable cause) {
    UnsatisfiedLinkError error = new UnsatisfiedLinkError(message);
    error.initCause(cause);
    return error;
  }
}
