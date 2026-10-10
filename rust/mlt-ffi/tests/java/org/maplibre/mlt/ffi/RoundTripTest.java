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

  @Test
  void builderOutputEqualsMvtConversionOfTheSamePointLayer() throws IOException {
    try (var options = new EncoderOptions(); var builder = new MltLayerBuilder("layer", 4096)) {
      int key = builder.addProperty("key");
      builder.beginFeature(GeometryType.POINT, 1L);
      builder.addPoints(new int[] {25, 17});
      builder.setBool(key, true);

      assertArrayEquals(Converter.mvtToMlt(fixture("point-boolean.mvt"), options), encode(builder, options));
    }
  }

  @Test
  void multipolygonBuilderOutputInV2ReencodesToTheSameMlt() throws IOException {
    try (var options = new EncoderOptions().wireVersion(WireVersion.V02);
      var builder = new MltLayerBuilder("layer", 4096)) {
      int key = builder.addProperty("key");
      builder.beginFeature(GeometryType.MULTI_POLYGON, 1L);
      builder.addExteriorRing(new int[] {0, 0, 10, 0, 10, 10, 0, 10, 0, 0});
      builder.addExteriorRing(new int[] {20, 20, 30, 20, 30, 30, 20, 30, 20, 20});
      builder.setBool(key, true);
      byte[] built = encode(builder, options);

      byte[] mvt = Converter.mltToMvt(built);

      assertArrayEquals(built, Converter.mvtToMlt(mvt, options));
    }
  }

  @Test
  void mvtDecodedFromBuilderOutputReencodesToTheSameMlt() throws IOException {
    try (var options = new EncoderOptions(); var builder = new MltLayerBuilder("layer", 4096)) {
      int flag = builder.addProperty("flag");
      int name = builder.addProperty("name");
      builder.beginFeature(GeometryType.LINE_STRING, 3L);
      builder.addLine(new int[] {0, 0, 10, 10, 20, 5});
      builder.setBool(flag, false);
      builder.setString(name, "alpha");
      builder.beginFeature(GeometryType.MULTI_LINE_STRING, 4L);
      builder.addLine(new int[] {1, 1, 2, 2});
      builder.addLine(new int[] {5, 5, 6, 7, 8, 9});
      builder.setBool(flag, true);
      builder.setString(name, "beta");
      byte[] built = encode(builder, options);

      assertArrayEquals(built, Converter.mvtToMlt(Converter.mltToMvt(built), options));
    }
  }

  @Test
  void integerAndFloatValuesOfOneKeyEncodeLikeDoubles() {
    try (var options = new EncoderOptions();
      var mixed = new MltLayerBuilder("layer", 4096);
      var doubles = new MltLayerBuilder("layer", 4096)) {
      int mixedKey = mixed.addProperty("key");
      mixed.beginFeature(GeometryType.POINT);
      mixed.addPoints(new int[] {1, 1});
      mixed.setLong(mixedKey, 3L);
      mixed.beginFeature(GeometryType.POINT);
      mixed.addPoints(new int[] {2, 2});
      mixed.setFloat(mixedKey, 1.5f);
      int doublesKey = doubles.addProperty("key");
      doubles.beginFeature(GeometryType.POINT);
      doubles.addPoints(new int[] {1, 1});
      doubles.setDouble(doublesKey, 3.0);
      doubles.beginFeature(GeometryType.POINT);
      doubles.addPoints(new int[] {2, 2});
      doubles.setDouble(doublesKey, 1.5);

      assertArrayEquals(encode(doubles, options), encode(mixed, options));
    }
  }

  @Test
  void booleanAndIntegerValuesOfOneKeyEncodeLikeText() {
    try (var options = new EncoderOptions();
      var mixed = new MltLayerBuilder("layer", 4096);
      var text = new MltLayerBuilder("layer", 4096)) {
      int mixedKey = mixed.addProperty("key");
      mixed.beginFeature(GeometryType.POINT);
      mixed.addPoints(new int[] {1, 1});
      mixed.setBool(mixedKey, true);
      mixed.beginFeature(GeometryType.POINT);
      mixed.addPoints(new int[] {2, 2});
      mixed.setLong(mixedKey, 5L);
      int textKey = text.addProperty("key");
      text.beginFeature(GeometryType.POINT);
      text.addPoints(new int[] {1, 1});
      text.setString(textKey, "true");
      text.beginFeature(GeometryType.POINT);
      text.addPoints(new int[] {2, 2});
      text.setString(textKey, "5");

      assertArrayEquals(encode(text, options), encode(mixed, options));
    }
  }

  @Test
  void layersAppendedToOneBufferEqualTheirSeparateEncodings() {
    try (var options = new EncoderOptions(); var builder = new MltLayerBuilder("first", 4096);
      var tile = new MltBuffer()) {
      builder.beginFeature(GeometryType.POINT);
      builder.addPoints(new int[] {1, 1});
      builder.encodeInto(options, tile);
      byte[] first = encode(builder, options);
      builder.reset("second", 4096);
      builder.beginFeature(GeometryType.POINT);
      builder.addPoints(new int[] {2, 2});
      builder.encodeInto(options, tile);
      byte[] second = encode(builder, options);

      var expected = new ByteArrayOutputStream();
      expected.writeBytes(first);
      expected.writeBytes(second);
      assertArrayEquals(expected.toByteArray(), tile.toByteArray());
    }
  }

  @Test
  void clearedBufferIsEmpty() {
    try (var options = new EncoderOptions(); var builder = new MltLayerBuilder("layer", 4096);
      var out = new MltBuffer()) {
      builder.beginFeature(GeometryType.POINT);
      builder.addPoints(new int[] {1, 1});
      builder.encodeInto(options, out);

      out.clear();

      assertEquals(0, out.length());
    }
  }

  @Test
  void rangeOverloadAddsOnlyTheRequestedCoordinates() {
    try (var options = new EncoderOptions(); var whole = new MltLayerBuilder("layer", 4096);
      var ranged = new MltLayerBuilder("layer", 4096)) {
      whole.beginFeature(GeometryType.LINE_STRING);
      whole.addLine(new int[] {1, 2, 3, 4});
      ranged.beginFeature(GeometryType.LINE_STRING);
      ranged.addLine(new int[] {9, 9, 1, 2, 3, 4, 9, 9}, 2, 4);

      assertArrayEquals(encode(whole, options), encode(ranged, options));
    }
  }

  @Test
  void geometryBeforeAFeatureBeginsIsAnInvalidFeature() {
    try (var builder = new MltLayerBuilder("layer", 4096)) {
      var error = assertThrows(MltException.class, () -> builder.addPoints(new int[] {1, 1}));

      assertEquals(MltException.Kind.INVALID_FEATURE, error.kind());
      assertEquals("no feature begun", error.getMessage());
    }
  }

  @Test
  void oddCoordinateCountIsAnInvalidFeature() {
    try (var builder = new MltLayerBuilder("layer", 4096)) {
      builder.beginFeature(GeometryType.LINE_STRING);

      var error = assertThrows(MltException.class, () -> builder.addLine(new int[] {1, 2, 3}));

      assertEquals(MltException.Kind.INVALID_FEATURE, error.kind());
      assertEquals("coordinates must come in x, y pairs", error.getMessage());
    }
  }

  @Test
  void undeclaredPropertyKeyIsAnInvalidFeature() {
    try (var builder = new MltLayerBuilder("layer", 4096)) {
      builder.beginFeature(GeometryType.POINT);

      var error = assertThrows(MltException.class, () -> builder.setLong(3, 1));

      assertEquals(MltException.Kind.INVALID_FEATURE, error.kind());
      assertEquals("unknown property key 3", error.getMessage());
    }
  }

  @Test
  void lineOnAPointFeatureFailsToEncode() {
    try (var options = new EncoderOptions(); var builder = new MltLayerBuilder("layer", 4096)) {
      builder.beginFeature(GeometryType.POINT);
      builder.addLine(new int[] {1, 1, 2, 2});

      var error = assertThrows(MltException.class, () -> encode(builder, options));

      assertEquals(MltException.Kind.ENCODING_FAILED, error.kind());
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

    assertThrows(IllegalStateException.class, () -> builder.addProperty("key"));
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
      int name = builder.addProperty("name");
      for (int f = 0; f < 50; f++) {
        builder.beginFeature(GeometryType.POINT, (long) f);
        builder.addPoints(new int[] {f, f * 2});
        builder.setString(name, "feature " + f);
      }
      return encode(builder, options);
    }
  }
}
