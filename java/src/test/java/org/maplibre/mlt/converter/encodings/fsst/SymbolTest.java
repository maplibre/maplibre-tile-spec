package org.maplibre.mlt.converter.encodings.fsst;

import static org.junit.jupiter.api.Assertions.*;

import java.nio.ByteBuffer;
import java.util.Arrays;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

class SymbolTest {
  Symbol s1 = Symbol.of(1);
  Symbol s2 = Symbol.of(2);
  Symbol s3 = Symbol.of(3);
  Symbol s12 = Symbol.concat(s1, s2);
  Symbol s12_3 = Symbol.concat(Symbol.concat(s1, s2), s3);
  Symbol s1_23 = Symbol.concat(s1, Symbol.concat(s2, s3));
  Symbol s12121212 = Symbol.concat(Symbol.concat(s12, s12), Symbol.concat(s12, s12));

  @Test
  void testBytes() {
    assertEquals(1, s1.length());
    assertEquals(1, s2.length());
    assertSymbol(s1, 1);
    assertSymbol(s2, 2);
    assertEquals(2, s12.length());
    assertSymbol(s12, 1, 2);
    assertEquals(3, s12_3.length());
    assertEquals(3, s1_23.length());
    assertEquals(s12_3.hashCode(), s1_23.hashCode());
    assertSymbol(s12_3, 1, 2, 3);
    assertSymbol(s1_23, 1, 2, 3);
    assertEquals(8, s12121212.length());
    assertSymbol(s12121212, 1, 2, 1, 2, 1, 2, 1, 2);
  }

  @Test
  void testMatch() {
    assertTrue(s1.match(ByteBuffer.wrap(new byte[] {1}), 0, 0));
    assertFalse(s2.match(ByteBuffer.wrap(new byte[] {1}), 0, 0));
    assertTrue(s2.match(ByteBuffer.wrap(new byte[] {1, 2}), 1, 0));
    assertFalse(s1.match(ByteBuffer.wrap(new byte[] {1, 2}), 1, 0));
  }

  @ParameterizedTest
  @ValueSource(bytes = {1, (byte) 255})
  void testMatch(byte first) {
    var matcher = Symbol.concat(Symbol.of(first), Symbol.of(2));
    byte[] bytesMatching = new byte[] {first, 2, 3, 4, 5, 6, 7, 8, 9, 10};
    byte[] bytesNotMatching = new byte[] {first, 3, 3, 4, 5, 6, 7, 8, 9, 10};
    for (int i = 2; i < 10; i++) {
      ByteBuffer bufMatch = ByteBuffer.wrap(bytesMatching, 0, i);
      ByteBuffer bufNoMatch = ByteBuffer.wrap(bytesNotMatching, 0, i);
      assertTrue(matcher.match(bufMatch, 0, 0), "expected match " + i);
      assertFalse(matcher.match(bufNoMatch, 0, 0), "expected no match " + i);
      assertFalse(matcher.match(bufMatch, 1, 0), "expected match with offset " + i);
    }
  }

  @Test
  void testCompare() {
    assertEquals(-1, s1.compareTo(s2));
    assertEquals(1, s2.compareTo(s1));
    assertEquals(0, s1.compareTo(Symbol.of(1)));

    assertEquals(-1, s12.compareTo(Symbol.concat(s1, s3)));
    // longer symbol must compare first
    assertEquals(-1, s12_3.compareTo(s12));
  }

  private static void assertSymbol(Symbol s, int... ints) {
    assertEquals(ints.length, s.length());
    byte[] bytes = new byte[ints.length];
    for (int i = 0; i < ints.length; i++) {
      bytes[i] = (byte) ints[i];
    }
    assertEquals(ints[0], s.first());
    assertEquals(Arrays.hashCode(bytes), s.hashCode());
    assertArrayEquals(
        bytes,
        s.bytes(),
        "Expected: %s Actual: %s".formatted(Arrays.toString(bytes), Arrays.toString(s.bytes())));
  }
}
