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
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

class NativeLibraryTest {

  private static final String OS = System.getProperty("os.name");
  private static final String ARCH = System.getProperty("os.arch");

  @Test
  void packagedLibraryIsExtractedFromTheJarWhenNoOverrideIsSet() {
    assumeTrue(System.getProperty(NativeLibrary.LIBRARY_PROPERTY) == null);

    NativeLibrary.load();
    Path loaded = NativeLibrary.loadedFrom();

    assertNotNull(loaded);
    String name = loaded.getFileName().toString();
    assertTrue(name.startsWith(NativeLibrary.TEMP_PREFIX) && name.endsWith(NativeLibrary.fileName(OS)), name);
    assertTrue(Files.exists(loaded.resolveSibling(name + NativeLibrary.LOCK_SUFFIX)));
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
