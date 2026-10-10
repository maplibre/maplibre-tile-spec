package org.maplibre.mlt.ffi;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import org.junit.jupiter.api.Test;

class RoundTripTest {

  private static final Path FIXTURES = Path.of("../../../../test/fixtures/simple");

  private static byte[] fixture(String name) throws IOException {
    return Files.readAllBytes(FIXTURES.resolve(name));
  }

  private static byte[] encode(MltLayerBuilder builder, EncoderOptions options) {
    try (var out = new MltBuffer()) {
      builder.encodeInto(options, out);
      return out.toByteArray();
    }
  }

  @Test
  void packagedNativeLibraryIsExtractedFromTheJarWhenNoOverrideIsSet() {
    NativeLibrary.load();

    Path loaded = NativeLibrary.loadedFrom();

    assertNull(System.getProperty(NativeLibrary.LIBRARY_PROPERTY));
    assertNotNull(loaded);
    assertEquals(NativeLibrary.fileName(System.getProperty("os.name")), loaded.getFileName().toString());
    assertTrue(loaded.toAbsolutePath().getParent().getFileName().toString().startsWith("mlt-ffi"));
  }

  @Test
  void packagedLibraryResourceLivesBelowThePackageSoTheKotlinJarCannotShadowIt() {
    String os = System.getProperty("os.name");
    String file = NativeLibrary.resourcePrefix(os, System.getProperty("os.arch"), false) + "/"
      + NativeLibrary.fileName(os);

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
    var error = assertThrows(UnsatisfiedLinkError.class, () -> NativeLibrary.resourcePrefix("Mac OS X", "x86_64", false));

    assertEquals("No MLT native library is packaged for Mac OS X x86_64, set -Dmlt.ffi.library to one",
      error.getMessage());
  }

  private static int zigzag(int value) {
    return (value << 1) ^ (value >> 31);
  }

  private static int[] point(int x, int y) {
    return new int[] {9, zigzag(x), zigzag(y)};
  }

  private static int[] commands(boolean close, int[]... parts) {
    var commands = new ArrayList<Integer>();
    int x = 0;
    int y = 0;
    for (int[] part : parts) {
      for (int i = 0; i < part.length; i += 2) {
        if (i == 0) {
          commands.add(9);
        } else if (i == 2) {
          commands.add(2 | ((part.length / 2 - 1) << 3));
        }
        commands.add(zigzag(part[i] - x));
        commands.add(zigzag(part[i + 1] - y));
        x = part[i];
        y = part[i + 1];
      }
      if (close) {
        commands.add(15);
      }
    }
    return commands.stream().mapToInt(Integer::intValue).toArray();
  }

  @Test
  void builderOutputEqualsMvtConversionOfTheSamePointLayer() throws IOException {
    try (var options = new EncoderOptions(); var builder = new MltLayerBuilder("layer", 4096)) {
      builder.beginMvtFeature(MvtGeometryType.POINT, point(25, 17), 1L);
      builder.setBool("key", true);

      assertArrayEquals(Converter.mvtToMlt(fixture("point-boolean.mvt"), options), encode(builder, options));
    }
  }

  @Test
  void multipolygonBuilderOutputInV2ReencodesToTheSameMlt() throws IOException {
    try (var options = new EncoderOptions().wireVersion(WireVersion.V02);
      var builder = new MltLayerBuilder("layer", 4096)) {
      builder.beginMvtFeature(MvtGeometryType.POLYGON,
        commands(true, new int[] {0, 0, 10, 0, 10, 10, 0, 10}, new int[] {20, 20, 30, 20, 30, 30, 20, 30}), 1L);
      builder.setBool("key", true);
      byte[] built = encode(builder, options);

      byte[] mvt = Converter.mltToMvt(built);

      assertArrayEquals(built, Converter.mvtToMlt(mvt, options));
    }
  }

  @Test
  void mvtDecodedFromBuilderOutputReencodesToTheSameMlt() throws IOException {
    try (var options = new EncoderOptions(); var builder = new MltLayerBuilder("layer", 4096)) {
      builder.beginMvtFeature(MvtGeometryType.LINE_STRING, commands(false, new int[] {0, 0, 10, 10, 20, 5}), 3L);
      builder.setBool("flag", false);
      builder.setString("name", "alpha");
      builder.beginMvtFeature(MvtGeometryType.LINE_STRING,
        commands(false, new int[] {1, 1, 2, 2}, new int[] {5, 5, 6, 7, 8, 9}), 4L);
      builder.setBool("flag", true);
      builder.setString("name", "beta");
      byte[] built = encode(builder, options);

      assertArrayEquals(built, Converter.mvtToMlt(Converter.mltToMvt(built), options));
    }
  }

  @Test
  void integerAndFloatValuesOfOneKeyEncodeLikeDoubles() {
    try (var options = new EncoderOptions();
      var mixed = new MltLayerBuilder("layer", 4096);
      var doubles = new MltLayerBuilder("layer", 4096)) {
      mixed.beginMvtFeature(MvtGeometryType.POINT, point(1, 1));
      mixed.setLong("key", 3L);
      mixed.beginMvtFeature(MvtGeometryType.POINT, point(2, 2));
      mixed.setFloat("key", 1.5f);
      doubles.beginMvtFeature(MvtGeometryType.POINT, point(1, 1));
      doubles.setDouble("key", 3.0);
      doubles.beginMvtFeature(MvtGeometryType.POINT, point(2, 2));
      doubles.setDouble("key", 1.5);

      assertArrayEquals(encode(doubles, options), encode(mixed, options));
    }
  }

  @Test
  void booleanAndIntegerValuesOfOneKeyEncodeLikeText() {
    try (var options = new EncoderOptions();
      var mixed = new MltLayerBuilder("layer", 4096);
      var text = new MltLayerBuilder("layer", 4096)) {
      mixed.beginMvtFeature(MvtGeometryType.POINT, point(1, 1));
      mixed.setBool("key", true);
      mixed.beginMvtFeature(MvtGeometryType.POINT, point(2, 2));
      mixed.setLong("key", 5L);
      text.beginMvtFeature(MvtGeometryType.POINT, point(1, 1));
      text.setString("key", "true");
      text.beginMvtFeature(MvtGeometryType.POINT, point(2, 2));
      text.setString("key", "5");

      assertArrayEquals(encode(text, options), encode(mixed, options));
    }
  }

  @Test
  void layersAppendedToOneBufferEqualTheirSeparateEncodings() {
    try (var options = new EncoderOptions(); var builder = new MltLayerBuilder("first", 4096);
      var tile = new MltBuffer()) {
      builder.beginMvtFeature(MvtGeometryType.POINT, point(1, 1));
      builder.encodeInto(options, tile);
      byte[] first = encode(builder, options);
      builder.reset("second", 4096);
      builder.beginMvtFeature(MvtGeometryType.POINT, point(2, 2));
      builder.encodeInto(options, tile);
      byte[] second = encode(builder, options);

      var expected = new ByteArrayOutputStream();
      expected.writeBytes(first);
      expected.writeBytes(second);
      assertArrayEquals(expected.toByteArray(), tile.toByteArray());
    }
  }

  @Test
  void resetLayerWithOtherPropertiesEncodesLikeAFreshBuilder() {
    try (var options = new EncoderOptions(); var reused = new MltLayerBuilder("layer", 4096);
      var fresh = new MltLayerBuilder("layer", 4096)) {
      reused.beginMvtFeature(MvtGeometryType.POINT, point(1, 1));
      reused.setLong("first", 1L);
      reused.setLong("second", 2L);
      reused.reset("layer", 4096);
      reused.beginMvtFeature(MvtGeometryType.POINT, point(2, 2));
      reused.setString("second", "two");
      fresh.beginMvtFeature(MvtGeometryType.POINT, point(2, 2));
      fresh.setString("second", "two");

      assertArrayEquals(encode(fresh, options), encode(reused, options));
    }
  }

  @Test
  void clearedBufferIsEmpty() {
    try (var options = new EncoderOptions(); var builder = new MltLayerBuilder("layer", 4096);
      var out = new MltBuffer()) {
      builder.beginMvtFeature(MvtGeometryType.POINT, point(1, 1));
      builder.encodeInto(options, out);

      out.clear();

      assertEquals(0, out.length());
    }
  }

  @Test
  void propertyBeforeAFeatureBeginsIsAnInvalidFeature() {
    try (var builder = new MltLayerBuilder("layer", 4096)) {
      var error = assertThrows(MltException.class, () -> builder.setLong("key", 1));

      assertEquals(MltException.Kind.INVALID_FEATURE, error.kind());
      assertEquals("no feature begun", error.getMessage());
    }
  }

  @Test
  void truncatedMvtCommandsAreAnInvalidFeature() {
    try (var builder = new MltLayerBuilder("layer", 4096)) {
      var error = assertThrows(MltException.class,
        () -> builder.beginMvtFeature(MvtGeometryType.LINE_STRING, new int[] {9, 2, 2, 18, 4}));

      assertEquals(MltException.Kind.INVALID_FEATURE, error.kind());
      assertEquals("invalid MVT geometry: ends inside a command", error.getMessage());
    }
  }

  @Test
  void lineToInAPointGeometryIsAnInvalidFeature() {
    try (var builder = new MltLayerBuilder("layer", 4096)) {
      var error = assertThrows(MltException.class,
        () -> builder.beginMvtFeature(MvtGeometryType.POINT, new int[] {9, 2, 2, 10, 4, 4}));

      assertEquals(MltException.Kind.INVALID_FEATURE, error.kind());
      assertEquals("invalid MVT geometry: a LineTo outside a line or ring", error.getMessage());
    }
  }

  @Test
  void nonPositiveExtentIsRejected() {
    assertThrows(IllegalArgumentException.class, () -> new MltLayerBuilder("layer", 0));
  }

  @Test
  void closedBuilderRejectsFurtherCalls() {
    var builder = new MltLayerBuilder("layer", 4096);
    builder.close();

    assertThrows(IllegalStateException.class, () -> builder.setLong("key", 1));
  }

  @Test
  void decodingGarbageReportsInvalidInputWithTheParserMessage() {
    var error = assertThrows(MltException.class, () -> Converter.mltToMvt(new byte[] {(byte) 0xff}));

    assertEquals(MltException.Kind.INVALID_INPUT, error.kind());
    assertEquals("buffer underflow: needed 2 bytes, but only 1 remain", error.getMessage());
  }

  @Test
  void encodingGarbageReportsEncodingFailedWithTheParserMessage() {
    try (var options = new EncoderOptions()) {
      var error = assertThrows(MltException.class, () -> Converter.mvtToMlt(new byte[] {(byte) 0xff}, options));

      assertEquals(MltException.Kind.ENCODING_FAILED, error.kind());
      assertEquals("MVT error: protobuf decode error: unexpected end of buffer", error.getMessage());
    }
  }

  @Test
  void decodingWithATinyMemoryLimitReportsTheBudget() throws IOException {
    try (var options = new EncoderOptions()) {
      byte[] mlt = Converter.mvtToMlt(fixture("point-boolean.mvt"), options);

      var error = assertThrows(MltException.class, () -> Converter.mltToMvt(mlt, 1));

      assertEquals(MltException.Kind.INVALID_INPUT, error.kind());
      assertEquals("memory limit exceeded: limit=1, used=0, requested=8", error.getMessage());
    }
  }

  @Test
  void decodingWithAGenerousMemoryLimitMatchesTheDefault() throws IOException {
    try (var options = new EncoderOptions()) {
      byte[] mlt = Converter.mvtToMlt(fixture("point-boolean.mvt"), options);

      assertArrayEquals(Converter.mltToMvt(mlt), Converter.mltToMvt(mlt, 1L << 30));
    }
  }

  @Test
  void memoryLimitAboveUnsignedIntIsRejected() {
    assertThrows(IllegalArgumentException.class, () -> Converter.mltToMvt(new byte[0], 1L << 32));
  }

  @Test
  void builderPerThreadEncodesTheSameBytesAsOneThread() throws Exception {
    byte[] reference = encodeLayer();
    var failures = new ArrayList<Throwable>();
    var threads = new ArrayList<Thread>();
    for (int t = 0; t < 8; t++) {
      var thread = new Thread(() -> {
        try {
          for (int i = 0; i < 200; i++) {
            assertArrayEquals(reference, encodeLayer());
          }
        } catch (Throwable e) {
          synchronized (failures) {
            failures.add(e);
          }
        }
      });
      threads.add(thread);
      thread.start();
    }
    for (var thread : threads) {
      thread.join();
    }

    assertEquals(List.of(), failures);
  }

  private static byte[] encodeLayer() {
    try (var options = new EncoderOptions(); var builder = new MltLayerBuilder("layer", 4096)) {
      for (int f = 0; f < 50; f++) {
        builder.beginMvtFeature(MvtGeometryType.POINT, point(f, f * 2), (long) f);
        builder.setString("name", "feature " + f);
      }
      return encode(builder, options);
    }
  }
}
