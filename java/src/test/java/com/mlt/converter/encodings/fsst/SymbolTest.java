package com.mlt.converter.encodings.fsst;

import java.nio.ByteBuffer;
import java.util.Arrays;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

import static org.junit.jupiter.api.Assertions.*;

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
    assertSymbol(s12_3, 1, 2, 3);
    assertSymbol(s1_23, 1, 2, 3);
    assertEquals(8, s12121212.length());
    assertSymbol(s12121212, 1, 2, 1, 2, 1, 2, 1, 2);
  }

  @ParameterizedTest
  @ValueSource(bytes = {1, (byte) 255})
  void testMatch(byte first) {
    ByteBuffer text23 = ByteBuffer.wrap(new byte[] {1});
    assertTrue(s1.match(text23, 0, 0));
    ByteBuffer text22 = ByteBuffer.wrap(new byte[] {1});
    assertFalse(s2.match(text22, 0, 0));
    ByteBuffer text21 = ByteBuffer.wrap(new byte[] {1, 2});
    assertTrue(s2.match(text21, 1, 0));
    ByteBuffer text20 = ByteBuffer.wrap(new byte[] {1, 2});
    assertFalse(s1.match(text20, 1, 0));

    var matcher = Symbol.concat(Symbol.of(first), Symbol.of(2));
    ByteBuffer text19 = ByteBuffer.wrap(new byte[] {first, 2});
    assertTrue(matcher.match(text19, 0, 0));
    ByteBuffer text18 = ByteBuffer.wrap(new byte[] {first, 2, 3});
    assertTrue(matcher.match(text18, 0, 0));
    ByteBuffer text17 = ByteBuffer.wrap(new byte[] {first, 2, 3, 4});
    assertTrue(matcher.match(text17, 0, 0));
    ByteBuffer text16 = ByteBuffer.wrap(new byte[] {first, 2, 3, 4, 5});
    assertTrue(matcher.match(text16, 0, 0));
    ByteBuffer text15 = ByteBuffer.wrap(new byte[] {first, 2, 3, 4, 5, 6});
    assertTrue(matcher.match(text15, 0, 0));
    ByteBuffer text14 = ByteBuffer.wrap(new byte[] {first, 2, 3, 4, 5, 6, 7});
    assertTrue(matcher.match(text14, 0, 0));
    ByteBuffer text13 = ByteBuffer.wrap(new byte[] {first, 2, 3, 4, 5, 6, 7, 8});
    assertTrue(matcher.match(text13, 0, 0));
    ByteBuffer text12 = ByteBuffer.wrap(new byte[] {first, 2, 3, 4, 5, 6, 7, 8, 9});
    assertTrue(matcher.match(text12, 0, 0));
    ByteBuffer text11 = ByteBuffer.wrap(new byte[] {first, 2, 3, 4, 5, 6, 7, 8, 9, 10});
    assertTrue(matcher.match(text11, 0, 0));
    ByteBuffer text10 = ByteBuffer.wrap(new byte[] {first, 3, 3, 4, 5, 6, 7, 8, 9, 10});
    assertFalse(matcher.match(text10, 0, 0));
    ByteBuffer text9 = ByteBuffer.wrap(new byte[] {first});
    assertFalse(matcher.match(text9, 0, 0));

    ByteBuffer text8 = ByteBuffer.wrap(new byte[] {first, 2});
    assertFalse(matcher.match(text8, 1, 0));
    ByteBuffer text7 = ByteBuffer.wrap(new byte[] {first, 2, 3});
    assertFalse(matcher.match(text7, 1, 0));
    ByteBuffer text6 = ByteBuffer.wrap(new byte[] {first, 2, 3, 4});
    assertFalse(matcher.match(text6, 1, 0));
    ByteBuffer text5 = ByteBuffer.wrap(new byte[] {first, 2, 3, 4, 5});
    assertFalse(matcher.match(text5, 1, 0));
    ByteBuffer text4 = ByteBuffer.wrap(new byte[] {first, 2, 3, 4, 5, 6});
    assertFalse(matcher.match(text4, 1, 0));
    ByteBuffer text3 = ByteBuffer.wrap(new byte[] {first, 2, 3, 4, 5, 6, 7});
    assertFalse(matcher.match(text3, 1, 0));
    ByteBuffer text2 = ByteBuffer.wrap(new byte[] {first, 2, 3, 4, 5, 6, 7, 8});
    assertFalse(matcher.match(text2, 1, 0));
    ByteBuffer text1 = ByteBuffer.wrap(new byte[] {first, 2, 3, 4, 5, 6, 7, 8, 9});
    assertFalse(matcher.match(text1, 1, 0));
    ByteBuffer text = ByteBuffer.wrap(new byte[] {first, 2, 3, 4, 5, 6, 7, 8, 9, 10});
    assertFalse(matcher.match(text, 1, 0));
  }

  @Test
  void testCompare() {
    assertEquals(-1, s1.compareTo(s2));
    assertEquals(1, s2.compareTo(s1));
    assertEquals(0, s1.compareTo(s1));

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
    assertArrayEquals(
        bytes,
        s.bytes(),
        "Expected: %s Actual: %s".formatted(Arrays.toString(bytes), Arrays.toString(s.bytes())));
  }
}
