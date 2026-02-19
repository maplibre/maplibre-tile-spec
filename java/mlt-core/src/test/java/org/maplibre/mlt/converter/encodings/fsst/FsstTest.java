package org.maplibre.mlt.converter.encodings.fsst;

import static org.junit.jupiter.api.Assertions.*;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Arrays;
import java.util.List;
import nl.bartlouwers.fsst.SymbolTable;
import org.junit.jupiter.api.AfterAll;
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
  void decode_simpleString_ValidEncodedAndDecoded() {
    test("AAAAAAABBBAAACCdddddEEEEEEfffEEEEAAAAAddddCC");
  }

  @Test
  void decodeLongRepeated() {
    test("AAAAAAABBBAAACCdddddEEEEEEfffEEEEAAAAAddddCC".repeat(100));
  }

  @Test
  void empty() {
    test("");
  }

  @Test
  void repeatedStrings() {
    for (int i = 1; i < 1000; i++) {
      test("a".repeat(i));
    }
  }

  @Test
  void allBytes() {
    byte[] toEncode = new byte[1000];
    for (int i = 0; i < toEncode.length; i++) {
      toEncode[i] = (byte) i;
    }
    test(toEncode);
  }

  static List<Path> tiles() throws IOException {
    try (var stream = Files.walk(Path.of("../..", "test"))) {
      return stream
          .filter(file -> Files.isRegularFile(file) && file.toString().endsWith(".mvt"))
          .toList();
    }
  }

  @ParameterizedTest
  @MethodSource("tiles")
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

  public static int weight(SymbolTable table) {
    return table.symbols().length + table.symbolLengths().length + table.compressedData().length;
  }

  @SuppressWarnings("deprecation")
  private static void test(byte[] input) {
    final var encodedJava = JAVA.encode(input);
    final var encodedJni = FsstJni.isLoaded() ? JNI.encode(input) : null;
    assertSymbolSortOrder(encodedJava);
    assertArrayEquals(input, JAVA.decode(encodedJava));

    if (encodedJni != null) {
      assertArrayEquals(input, JAVA.decode(encodedJni));

      final int maxAllowed = Math.max((int) (weight(encodedJni) * 1.02), weight(encodedJni) + 2);
      assertTrue(
          weight(encodedJava) <= maxAllowed,
          () ->
              """
                      Input: byte[%d]
                      Encoded java >5%% larger than JNI
                      Java: %s
                      JNI: %s
                      """
                  .formatted(input.length, encodedJava, encodedJni));

      // Native implementation doesn't produce the expected symbol order.  Do we care?
      // assertSymbolSortOrder(encodedJni);

      assertArrayEquals(input, JNI.decode(encodedJava));
      assertArrayEquals(input, JNI.decode(encodedJni));
      assertArrayEquals(
          input,
          JNI.decode(
              encodedJava.symbols(), encodedJava.symbolLengths(), encodedJava.compressedData()));
    }
    DEBUG.encode(input);
  }
}
