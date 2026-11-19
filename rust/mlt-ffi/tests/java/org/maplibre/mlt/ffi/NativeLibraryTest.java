package org.maplibre.mlt.ffi;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assumptions.assumeTrue;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.attribute.PosixFilePermissions;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

class NativeLibraryTest {

  private static final String OS = System.getProperty("os.name");
  private static final String ARCH = System.getProperty("os.arch");

  @Test
  void packagedLibraryIsExtractedFromTheJarWhenNoOverrideIsSet() throws IOException {
    assumeTrue(System.getProperty(NativeLibrary.LIBRARY_PROPERTY) == null);

    NativeLibrary.load();
    Path loaded = NativeLibrary.loadedFrom();

    assertNotNull(loaded);
    String name = loaded.getFileName().toString();
    assertTrue(name.startsWith(NativeLibrary.TEMP_PREFIX) && name.endsWith(NativeLibrary.fileName(OS)), name);
    assertTrue(Files.exists(loaded.resolveSibling(name + NativeLibrary.LOCK_SUFFIX)));
    if (loaded.getFileSystem().supportedFileAttributeViews().contains("posix")) {
      // other users of the shared temporary directory must not be able to swap the code
      assertEquals("rw-------", PosixFilePermissions.toString(Files.getPosixFilePermissions(loaded)));
    }
  }

  @Test
  void deleteStaleKeepsOnlyLibrariesWhoseLockExists(@TempDir Path tmp) throws IOException {
    Path stale = Files.createFile(tmp.resolve("mlt-ffi-a-libmlt_ffi.so"));
    Path live = Files.createFile(tmp.resolve("mlt-ffi-b-libmlt_ffi.so"));
    Path lock = Files.createFile(tmp.resolve("mlt-ffi-b-libmlt_ffi.so" + NativeLibrary.LOCK_SUFFIX));

    NativeLibrary.deleteStale(tmp.toFile());

    assertFalse(Files.exists(stale));
    assertTrue(Files.exists(live));
    assertTrue(Files.exists(lock));
  }

  @Test
  void exitedJvmLeavesNoLibraryOnceTheNextOneLoads(@TempDir Path tmp) throws IOException, InterruptedException {
    assumeTrue(System.getProperty(NativeLibrary.LIBRARY_PROPERTY) == null);

    Path first = loadInChildJvm(tmp);
    // Linux and macOS delete the library on exit, Windows only the lock while the DLL is still mapped
    assertFalse(Files.exists(first.resolveSibling(first.getFileName() + NativeLibrary.LOCK_SUFFIX)));
    Path second = loadInChildJvm(tmp);

    assertFalse(Files.exists(first));
    assertFalse(Files.exists(second.resolveSibling(second.getFileName() + NativeLibrary.LOCK_SUFFIX)));
  }

  /** Loads the library in a JVM with {@code tmp} as its temporary directory and returns the file it loaded. */
  private static Path loadInChildJvm(Path tmp) throws IOException, InterruptedException {
    Process process = new ProcessBuilder(Path.of(System.getProperty("java.home"), "bin", "java").toString(),
      "--enable-native-access=ALL-UNNAMED", "-Djava.io.tmpdir=" + tmp,
      "-cp", System.getProperty("java.class.path"), LoadAndExit.class.getName())
      .redirectErrorStream(true)
      .start();
    String output = new String(process.getInputStream().readAllBytes()).strip();
    assertEquals(0, process.waitFor(), output);
    return Path.of(output);
  }

  static final class LoadAndExit {
    public static void main(String[] args) {
      NativeLibrary.load();
      System.out.println(NativeLibrary.loadedFrom());
    }
  }

  @Test
  void packagedLibraryLivesBelowThePackageSoTheKotlinJarCannotShadowIt() {
    String file = NativeLibrary.resourcePrefix(OS, ARCH, false) + "/" + NativeLibrary.fileName(OS);

    assertNotNull(NativeLibrary.class.getResource("/org/maplibre/mlt/ffi/native/" + file));
    assertNull(NativeLibrary.class.getResource("/" + file));
  }

  @Test
  void platformsMapToTheirResourceDirectories() {
    assertEquals("linux-x86-64", NativeLibrary.resourcePrefix("Linux", "amd64", false));
    assertEquals("linux-x86-64-musl", NativeLibrary.resourcePrefix("Linux", "x86_64", true));
    assertEquals("linux-aarch64", NativeLibrary.resourcePrefix("Linux", "aarch64", false));
    assertEquals("linux-aarch64-musl", NativeLibrary.resourcePrefix("Linux", "aarch64", true));
    assertEquals("darwin-aarch64", NativeLibrary.resourcePrefix("Mac OS X", "aarch64", false));
    assertEquals("darwin-x86-64", NativeLibrary.resourcePrefix("Mac OS X", "x86_64", false));
    assertEquals("freebsd-x86-64", NativeLibrary.resourcePrefix("FreeBSD", "amd64", false));
    assertEquals("freebsd-aarch64", NativeLibrary.resourcePrefix("FreeBSD", "aarch64", false));
    assertEquals("win32-x86-64", NativeLibrary.resourcePrefix("Windows 11", "amd64", false));
    assertEquals("win32-aarch64", NativeLibrary.resourcePrefix("Windows 11", "aarch64", false));
  }

  @Test
  void unsupportedPlatformIsReportedWithTheOverrideProperty() {
    var error = assertThrows(UnsatisfiedLinkError.class,
      () -> NativeLibrary.resourcePrefix("Linux", "riscv64", false));

    assertEquals("No MLT native library is packaged for Linux riscv64, set -Dmlt.ffi.library to one",
      error.getMessage());
  }
}
