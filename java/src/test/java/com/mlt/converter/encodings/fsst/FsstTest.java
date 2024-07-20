package com.mlt.converter.encodings.fsst;

import static org.junit.jupiter.api.Assertions.*;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
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
  void decode_simpleString_ValidEncodedAndDecoded() throws IOException {
    test("AAAAAAABBBAAACCdddddEEEEEEfffEEEEAAAAAddddCC");
  }

  @Test
  void decodeLongRepeated() throws IOException {
    test("AAAAAAABBBAAACCdddddEEEEEEfffEEEEAAAAAddddCC".repeat(100));
  }

  @Test
  void empty() throws IOException {
    test("");
  }

  @Test
  void repeatedStrings() throws IOException {
    for (int i = 1; i < 1000; i++) {
      test("a".repeat(i));
    }
  }

  @Test
  void allBytes() throws IOException {
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
  void fsstEncodeTile(Path path) throws IOException {
    // stress-test FSST encoding by using it to encode raw tiles
    // ideally this would encode just the dictionaries, but it's close enough for now
    test(Files.readAllBytes(path));
  }

  private static void test(String input) throws IOException {
    test(input.getBytes(StandardCharsets.UTF_8));
  }

  private static void test(byte[] input) throws IOException {
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
    assertArrayEquals(input, JAVA.decode(encodedJava));
    assertArrayEquals(input, JAVA.decode(encodedJni));
    assertArrayEquals(input, JNI.decode(encodedJava));
    assertArrayEquals(input, JNI.decode(encodedJni));
    DEBUG.encode(input);
  }
}
