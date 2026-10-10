package org.maplibre.mlt.ffi;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assumptions.assumeTrue;

import java.nio.file.Path;
import org.junit.jupiter.api.Test;

class NativeLibraryTest {

  private static final String OS = System.getProperty("os.name");
  private static final String ARCH = System.getProperty("os.arch");

  @Test
  void packagedLibraryIsExtractedFromTheJarWhenNoOverrideIsSet() {
    assumeTrue(System.getProperty(NativeLibrary.LIBRARY_PROPERTY) == null);

    NativeLibrary.load();
    Path loaded = NativeLibrary.loadedFrom();

    assertNotNull(loaded);
    assertEquals(NativeLibrary.fileName(OS), loaded.getFileName().toString());
    assertTrue(loaded.toAbsolutePath().getParent().getFileName().toString().startsWith("mlt-ffi"));
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
    assertEquals("darwin-aarch64", NativeLibrary.resourcePrefix("Mac OS X", "aarch64", false));
    assertEquals("win32-x86-64", NativeLibrary.resourcePrefix("Windows 11", "amd64", false));
  }

  @Test
  void unsupportedPlatformIsReportedWithTheOverrideProperty() {
    var error = assertThrows(UnsatisfiedLinkError.class,
      () -> NativeLibrary.resourcePrefix("Mac OS X", "x86_64", false));

    assertEquals("No MLT native library is packaged for Mac OS X x86_64, set -Dmlt.ffi.library to one",
      error.getMessage());
  }
}
