package org.maplibre.mlt.ffi;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import com.onthegomap.planetiler.VectorTile;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Collections;
import java.util.concurrent.Callable;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.Executors;
import java.util.function.Consumer;
import org.junit.jupiter.api.Test;
import org.locationtech.jts.geom.util.AffineTransformation;
import org.locationtech.jts.io.ParseException;
import org.locationtech.jts.io.WKTReader;

class RoundTripTest {

  private static final Path FIXTURES = Path.of("../../../../test/fixtures/simple");
  private static final int EXTENT = 4096;
  private static final AffineTransformation TILE_UNITS_TO_PLANETILER_UNITS =
    AffineTransformation.scaleInstance(256d / EXTENT, 256d / EXTENT);

  private static byte[] fixture(String name) throws IOException {
    return Files.readAllBytes(FIXTURES.resolve(name));
  }

  private static byte[] encode(Consumer<MltEncoder> tile) {
    return encode(MltEncoderOptions.defaults(), tile);
  }

  private static byte[] encode(MltEncoderOptions options, Consumer<MltEncoder> tile) {
    try (var encoder = new MltEncoder(options)) {
      tile.accept(encoder);
      return encoder.toByteArray();
    }
  }

  private static VectorTile.VectorGeometry mvt(String wkt) {
    try {
      return VectorTile.encodeGeometry(TILE_UNITS_TO_PLANETILER_UNITS.transform(new WKTReader().read(wkt)));
    } catch (ParseException e) {
      throw new IllegalArgumentException(wkt, e);
    }
  }

  private static MvtGeometryType type(VectorTile.VectorGeometry geometry) {
    return switch (geometry.geomType()) {
      case POINT -> MvtGeometryType.POINT;
      case LINE -> MvtGeometryType.LINE_STRING;
      case POLYGON -> MvtGeometryType.POLYGON;
      case UNKNOWN -> throw new IllegalArgumentException(geometry.toString());
    };
  }

  private static void beginFeature(MltEncoder encoder, String wkt) {
    var geometry = mvt(wkt);
    encoder.beginFeature(type(geometry), geometry.commands());
  }

  private static void beginFeature(MltEncoder encoder, String wkt, long id) {
    var geometry = mvt(wkt);
    encoder.beginFeature(type(geometry), geometry.commands(), id);
  }

  @Test
  void pointWithABooleanEncodesLikeTheConvertedMvtFixture() throws IOException {
    byte[] built = encode(encoder -> {
      encoder.beginLayer("layer", EXTENT);
      beginFeature(encoder, "POINT (25 17)", 1L);
      encoder.setBool("key", true);
    });

    assertArrayEquals(MltConverter.mvtToMlt(fixture("point-boolean.mvt")), built);
  }

  @Test
  void linesWithPropertiesReencodeToTheSameMlt() {
    byte[] built = encode(encoder -> {
      encoder.beginLayer("layer", EXTENT);
      beginFeature(encoder, "LINESTRING (0 0, 10 10, 20 5)", 3L);
      encoder.setBool("flag", false);
      encoder.setString("name", "alpha");
      beginFeature(encoder, "MULTILINESTRING ((1 1, 2 2), (5 5, 6 7, 8 9))", 4L);
      encoder.setBool("flag", true);
      encoder.setString("name", "beta");
    });

    assertArrayEquals(built, MltConverter.mvtToMlt(MltConverter.mltToMvt(built)));
  }

  @Test
  void polygonWithAHoleReencodesToTheSameMlt() {
    byte[] built = encode(encoder -> {
      encoder.beginLayer("layer", EXTENT);
      beginFeature(encoder, "POLYGON ((0 0, 100 0, 100 100, 0 100, 0 0), (20 20, 20 40, 40 40, 40 20, 20 20))");
      encoder.setString("kind", "park");
    });

    assertArrayEquals(built, MltConverter.mvtToMlt(MltConverter.mltToMvt(built)));
  }

  @Test
  void multipolygonInV2ReencodesToTheSameMlt() {
    var v2 = MltEncoderOptions.builder().wireVersion(MltWireVersion.V2).build();
    byte[] built = encode(v2, encoder -> {
      encoder.beginLayer("layer", EXTENT);
      beginFeature(encoder, "MULTIPOLYGON (((0 0, 10 0, 10 10, 0 10, 0 0)), ((20 20, 30 20, 30 30, 20 30, 20 20)))", 1L);
      encoder.setBool("key", true);
    });

    assertArrayEquals(built, MltConverter.mvtToMlt(MltConverter.mltToMvt(built), v2));
  }

  @Test
  void integerAndFloatValuesOfOneKeyEncodeLikeDoubles() {
    byte[] mixed = encode(encoder -> {
      encoder.beginLayer("layer", EXTENT);
      beginFeature(encoder, "POINT (1 1)");
      encoder.setLong("key", 3L);
      beginFeature(encoder, "POINT (2 2)");
      encoder.setFloat("key", 1.5f);
    });
    byte[] doubles = encode(encoder -> {
      encoder.beginLayer("layer", EXTENT);
      beginFeature(encoder, "POINT (1 1)");
      encoder.setDouble("key", 3.0);
      beginFeature(encoder, "POINT (2 2)");
      encoder.setDouble("key", 1.5);
    });

    assertArrayEquals(doubles, mixed);
  }

  @Test
  void booleanAndIntegerValuesOfOneKeyEncodeLikeText() {
    byte[] mixed = encode(encoder -> {
      encoder.beginLayer("layer", EXTENT);
      beginFeature(encoder, "POINT (1 1)");
      encoder.setBool("key", true);
      beginFeature(encoder, "POINT (2 2)");
      encoder.setLong("key", 5L);
    });
    byte[] text = encode(encoder -> {
      encoder.beginLayer("layer", EXTENT);
      beginFeature(encoder, "POINT (1 1)");
      encoder.setString("key", "true");
      beginFeature(encoder, "POINT (2 2)");
      encoder.setString("key", "5");
    });

    assertArrayEquals(text, mixed);
  }

  @Test
  void tileOfTwoLayersEqualsTheirSingleLayerTilesConcatenated() {
    byte[] first = encode(encoder -> {
      encoder.beginLayer("first", EXTENT);
      beginFeature(encoder, "POINT (1 1)");
    });
    byte[] second = encode(encoder -> {
      encoder.beginLayer("second", EXTENT);
      beginFeature(encoder, "POINT (2 2)");
    });
    byte[] both = encode(encoder -> {
      encoder.beginLayer("first", EXTENT);
      beginFeature(encoder, "POINT (1 1)");
      encoder.beginLayer("second", EXTENT);
      beginFeature(encoder, "POINT (2 2)");
    });

    var concatenated = new ByteArrayOutputStream();
    concatenated.writeBytes(first);
    concatenated.writeBytes(second);
    assertArrayEquals(concatenated.toByteArray(), both);
  }

  @Test
  void tileAfterAnotherWithOtherPropertiesEncodesLikeAFreshEncoder() {
    byte[] fresh = encode(encoder -> {
      encoder.beginLayer("layer", EXTENT);
      beginFeature(encoder, "POINT (2 2)");
      encoder.setString("second", "two");
    });
    byte[] reused = encode(encoder -> {
      encoder.beginLayer("layer", EXTENT);
      beginFeature(encoder, "POINT (1 1)");
      encoder.setLong("first", 1L);
      encoder.setLong("second", 2L);
      encoder.toByteArray();
      encoder.beginLayer("layer", EXTENT);
      beginFeature(encoder, "POINT (2 2)");
      encoder.setString("second", "two");
    });

    assertArrayEquals(fresh, reused);
  }

  @Test
  void tileWithoutLayersIsEmpty() {
    assertArrayEquals(new byte[0], encode(encoder -> {}));
  }

  @Test
  void encodersOnConcurrentThreadsEncodeTheSameBytes() throws Exception {
    byte[] reference = fiftyNamedPoints();

    Callable<byte[]> encodeOnce = RoundTripTest::fiftyNamedPoints;
    try (var threads = Executors.newFixedThreadPool(8)) {
      for (var tile : threads.invokeAll(Collections.nCopies(1600, encodeOnce))) {
        assertArrayEquals(reference, tile.get());
      }
    }
  }

  private static byte[] fiftyNamedPoints() {
    return encode(encoder -> {
      encoder.beginLayer("layer", EXTENT);
      for (int f = 0; f < 50; f++) {
        beginFeature(encoder, "POINT (" + f + " " + f * 2 + ")", f);
        encoder.setString("name", "feature " + f);
      }
    });
  }

  @Test
  void encoderUsedFromAnotherThreadIsRejected() {
    try (var encoder = new MltEncoder(); var threads = Executors.newSingleThreadExecutor()) {
      var error = assertThrows(ExecutionException.class,
        () -> threads.submit(() -> encoder.beginLayer("layer", EXTENT)).get());

      assertEquals(WrongThreadException.class, error.getCause().getClass());
    }
  }

  @Test
  void featureBeforeALayerBeginsIsRejected() {
    try (var encoder = new MltEncoder()) {
      var error = assertThrows(IllegalStateException.class, () -> beginFeature(encoder, "POINT (1 1)"));

      assertEquals("no layer begun", error.getMessage());
    }
  }

  @Test
  void featureAfterToByteArrayNeedsANewLayer() {
    try (var encoder = new MltEncoder()) {
      encoder.beginLayer("layer", EXTENT);
      encoder.toByteArray();

      var error = assertThrows(IllegalStateException.class, () -> beginFeature(encoder, "POINT (1 1)"));

      assertEquals("no layer begun", error.getMessage());
    }
  }

  @Test
  void propertyBeforeAFeatureBeginsIsAnInvalidFeature() {
    try (var encoder = new MltEncoder()) {
      encoder.beginLayer("layer", EXTENT);

      var error = assertThrows(MltException.class, () -> encoder.setLong("key", 1));

      assertEquals(MltException.Kind.INVALID_FEATURE, error.kind());
      assertEquals("no feature begun", error.getMessage());
    }
  }

  @Test
  void truncatedMvtCommandsAreAnInvalidFeature() {
    try (var encoder = new MltEncoder()) {
      encoder.beginLayer("layer", EXTENT);

      var error = assertThrows(MltException.class,
        () -> encoder.beginFeature(MvtGeometryType.LINE_STRING, new int[] {9, 2, 2, 18, 4}));

      assertEquals(MltException.Kind.INVALID_FEATURE, error.kind());
      assertEquals("invalid MVT geometry: ends inside a command", error.getMessage());
    }
  }

  @Test
  void lineToInAPointGeometryIsAnInvalidFeature() {
    try (var encoder = new MltEncoder()) {
      encoder.beginLayer("layer", EXTENT);

      var error = assertThrows(MltException.class,
        () -> encoder.beginFeature(MvtGeometryType.POINT, new int[] {9, 2, 2, 10, 4, 4}));

      assertEquals(MltException.Kind.INVALID_FEATURE, error.kind());
      assertEquals("invalid MVT geometry: a LineTo outside a line or ring", error.getMessage());
    }
  }

  @Test
  void zeroExtentIsRejected() {
    try (var encoder = new MltEncoder()) {
      var error = assertThrows(IllegalArgumentException.class, () -> encoder.beginLayer("layer", 0));

      assertEquals("extent must be positive: 0", error.getMessage());
    }
  }

  @Test
  void closedEncoderRejectsFurtherCalls() {
    var encoder = new MltEncoder();
    encoder.beginLayer("layer", EXTENT);
    encoder.close();

    var error = assertThrows(IllegalStateException.class, () -> encoder.setLong("key", 1));

    assertEquals("MltEncoder is closed", error.getMessage());
  }

  @Test
  void decodingGarbageReportsInvalidInputWithTheParserMessage() {
    var error = assertThrows(MltException.class, () -> MltConverter.mltToMvt(new byte[] {(byte) 0xff}));

    assertEquals(MltException.Kind.INVALID_INPUT, error.kind());
    assertEquals("buffer underflow: needed 2 bytes, but only 1 remain", error.getMessage());
  }

  @Test
  void encodingGarbageReportsEncodingFailedWithTheParserMessage() {
    var error = assertThrows(MltException.class, () -> MltConverter.mvtToMlt(new byte[] {(byte) 0xff}));

    assertEquals(MltException.Kind.ENCODING_FAILED, error.kind());
    assertEquals("MVT error: protobuf decode error: unexpected end of buffer", error.getMessage());
  }

  @Test
  void emptyMvtEncodesToAnEmptyTile() {
    assertArrayEquals(new byte[0], MltConverter.mvtToMlt(new byte[0]));
  }

  @Test
  void decodingWithATinyMemoryLimitReportsTheBudget() throws IOException {
    byte[] mlt = MltConverter.mvtToMlt(fixture("point-boolean.mvt"));

    var error = assertThrows(MltException.class, () -> MltConverter.mltToMvt(mlt, 1));

    assertEquals(MltException.Kind.INVALID_INPUT, error.kind());
    assertEquals("memory limit exceeded: limit=1, used=0, requested=8", error.getMessage());
  }

  @Test
  void decodingWithAGenerousMemoryLimitMatchesTheDefault() throws IOException {
    byte[] mlt = MltConverter.mvtToMlt(fixture("point-boolean.mvt"));

    assertArrayEquals(MltConverter.mltToMvt(mlt), MltConverter.mltToMvt(mlt, 1L << 30));
  }

  @Test
  void memoryLimitAboveUnsignedIntIsRejected() {
    var error = assertThrows(IllegalArgumentException.class, () -> MltConverter.mltToMvt(new byte[0], 1L << 32));

    assertEquals("maxBytes must fit an unsigned 32-bit integer: 4294967296", error.getMessage());
  }
}
