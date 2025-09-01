package org.maplibre.mlt.converter.encodings.fsst;

import static org.junit.jupiter.api.Assertions.*;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Arrays;
import java.util.List;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.Disabled;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.MethodSource;

class FsstTest {
  private static final Fsst JAVA = new FsstJava();
  private static final Fsst JNI = new FsstJni();
  private static final FsstDebug DEBUG = new FsstDebug();

  @AfterAll
  static void printStats() {
    FsstDebug.printStats();
  }

  @Test
  @Disabled
  void decode_simpleString_ValidEncodedAndDecoded() {
    test("AAAAAAABBBAAACCdddddEEEEEEfffEEEEAAAAAddddCC");
  }

  @Test
  @Disabled
  void decodeLongRepeated() {
    test("AAAAAAABBBAAACCdddddEEEEEEfffEEEEAAAAAddddCC".repeat(100));
  }

  @Test
  @Disabled
  void empty() {
    test("");
  }

  @Test
  @Disabled
  void repeatedStrings() {
    for (int i = 1; i < 1000; i++) {
      test("a".repeat(i));
    }
  }

  @Test
  @Disabled
  void allBytes() {
    byte[] toEncode = new byte[1000];
    for (int i = 0; i < toEncode.length; i++) {
      toEncode[i] = (byte) i;
    }
    test(toEncode);
  }

  static List<Path> tiles() throws IOException {
    try (var stream = Files.walk(Path.of("..", "test"))) {
      return stream
          .filter(file -> Files.isRegularFile(file) && file.toString().endsWith(".mvt"))
          .toList();
    }
  }

  @ParameterizedTest
  @MethodSource("tiles")
  @Disabled
  void fsstEncodeTile(Path path) throws IOException {
    // stress-test FSST encoding by using it to encode raw tiles
    // ideally this would encode just the dictionaries, but it's close enough for now
    test(Files.readAllBytes(path));
  }

  private static void test(String input) {
    test(input.getBytes(StandardCharsets.UTF_8));
  }

  private static void assertSymbolSortOrder(SymbolTable table) {
    int last = 2;
    boolean ones = false;
    for (int len : table.symbolLengths()) {
      if (len == 1) {
        ones = true;
      }
      if (ones) {
        assertEquals(
            1,
            len,
            () ->
                "Expected symbols sorted by length with single-byte symbols last, got: "
                    + Arrays.toString(table.symbolLengths()));
      } else {
        assertTrue(
            len >= last,
            () ->
                "Expected symbols sorted by length with single-byte symbols last, got: "
                    + Arrays.toString(table.symbolLengths()));
        last = len;
      }
    }
  }

  @SuppressWarnings("deprecation")
  private static void test(byte[] input) {
    var encodedJava = JAVA.encode(input);
    var encodedJni = JNI.encode(input);
    int maxAllowed = Math.max((int) (encodedJni.weight() * 1.02), encodedJni.weight() + 2);
    assertTrue(
        encodedJava.weight() <= maxAllowed,
        () ->
            """
            Input: byte[%d]
            Encoded java >5%% larger than JNI
            Java: %s
            JNI: %s
            """
                .formatted(input.length, encodedJava, encodedJni));
    assertSymbolSortOrder(encodedJni);
    assertSymbolSortOrder(encodedJava);
    assertArrayEquals(input, JAVA.decode(encodedJava));
    assertArrayEquals(input, JAVA.decode(encodedJni));
    assertArrayEquals(input, JNI.decode(encodedJava));
    assertArrayEquals(input, JNI.decode(encodedJni));
    assertArrayEquals(
        input,
        JNI.decode(
            encodedJava.symbols(), encodedJava.symbolLengths(), encodedJava.compressedData()));
    DEBUG.encode(input);
  }
}
