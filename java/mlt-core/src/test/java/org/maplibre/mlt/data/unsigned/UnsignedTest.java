package org.maplibre.mlt.data.unsigned;

import static org.junit.jupiter.api.Assertions.*;

import java.math.BigInteger;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

/** Tests for the {@link Unsigned} interface and its implementations */
class UnsignedTest {
  @Test
  void testU8OfValidValues() {
    final var u8Zero = U8.of(0);
    assertEquals((byte) 0, u8Zero.byteValue());
    assertEquals(0, u8Zero.intValue());
    assertEquals(0L, u8Zero.longValue());

    final var u8Max = U8.of(255);
    assertEquals((byte) 255, u8Max.byteValue());
    assertEquals(255, u8Max.intValue());
    assertEquals(255L, u8Max.longValue());

    final var u8Mid = U8.of(128);
    assertEquals((byte) 128, u8Mid.byteValue());
    assertEquals(128, u8Mid.intValue());
    assertEquals(128L, u8Mid.longValue());
  }

  @Test
  void testU8OfNegativeValue() {
    assertThrows(
        IllegalArgumentException.class, () -> U8.of(-1), "U8 should reject negative values");
  }

  @Test
  void testU8OfValueTooLarge() {
    assertThrows(IllegalArgumentException.class, () -> U8.of(256), "U8 should reject values > 255");
  }

  @ParameterizedTest
  @ValueSource(ints = {0, 1, 127, 128, 254, 255})
  void testU8ValidRange(int value) {
    final var u8 = U8.of(value);
    assertEquals(value, u8.intValue(), "U8 should preserve unsigned value as int");
    assertEquals((long) value, u8.longValue(), "U8 should preserve unsigned value as long");
  }

  @Test
  void testU8RecordProperties() {
    assertEquals(42, (int) U8.of(42).value(), "Record should expose value");
  }

  @Test
  void testU8ToString() {
    assertEquals("u8(0)", U8.of(0).toString());
    assertEquals("u8(255)", U8.of(255).toString());
    assertEquals("u8(42)", U8.of(42).toString());
  }

  @Test
  void testU8Equality() {
    final var u8a = U8.of(42);
    final var u8b = U8.of(42);
    final var u8c = U8.of(43);

    assertEquals(u8a, u8b, "U8 records with same value should be equal");
    assertNotEquals(u8a, u8c, "U8 records with different values should not be equal");
  }

  @Test
  void testU8ImplementsUnsigned() {
    final var u8 = U8.of(100);
    assertTrue(u8 instanceof Unsigned, "U8 should implement Unsigned");
    assertNotNull(u8.byteValue());
    assertNotNull(u8.intValue());
    assertNotNull(u8.longValue());
  }

  @Test
  void testU32OfValidValues() {
    // Test boundary values
    final var u32Zero = U32.of(0);
    assertEquals(0, u32Zero.intValue());
    assertEquals(0L, u32Zero.longValue());

    final var u32Max = U32.of(0xFFFFFFFFL);
    assertEquals(-1, u32Max.intValue()); // -1 in signed int representation
    assertEquals(0xFFFFFFFFL, u32Max.longValue()); // Correct unsigned value

    final var u32Mid = U32.of(0x80000000L);
    assertEquals(0x80000000L, u32Mid.longValue());
  }

  @Test
  void testU32OfNegativeValue() {
    assertThrows(
        IllegalArgumentException.class, () -> U32.of(-1), "U32 should reject negative values");
  }

  @Test
  void testU32OfValueTooLarge() {
    assertThrows(
        IllegalArgumentException.class,
        () -> U32.of(0x100000000L),
        "U32 should reject values > 2^32-1");
  }

  @ParameterizedTest
  @ValueSource(longs = {0L, 1L, 0x7FFFFFFFL, 0x80000000L, 0xFFFFFFFEL, 0xFFFFFFFFL})
  void testU32ValidRange(long value) {
    assertEquals(value, U32.of(value).longValue(), "U32 should preserve unsigned value as long");
  }

  @Test
  void testU32RecordProperties() {
    assertEquals(0x12345678, U32.of(0x12345678L).value(), "Record should expose value");
  }

  @Test
  void testU32ToString() {
    assertEquals("u32(0)", U32.of(0).toString());
    assertEquals("u32(4294967295)", U32.of(0xFFFFFFFFL).toString());
    assertEquals("u32(305419896)", U32.of(0x12345678L).toString());
  }

  @Test
  void testU32Equality() {
    final var u32a = U32.of(0x12345678L);
    final var u32b = U32.of(0x12345678L);
    final var u32c = U32.of(0x12345679L);

    assertEquals(u32a, u32b, "U32 records with same value should be equal");
    assertNotEquals(u32a, u32c, "U32 records with different values should not be equal");
  }

  @Test
  void testU32ImplementsUnsigned() {
    final var u32 = U32.of(1000000L);
    assertTrue(u32 instanceof Unsigned, "U32 should implement Unsigned");
    assertNotNull(u32.intValue());
    assertNotNull(u32.longValue());
  }

  @Test
  void testU32ByteValueReturnsNullWhenOutOfRange() {
    final var u32InRange = U32.of(100);
    assertNotNull(u32InRange.byteValue(), "U32 with value in byte range should return non-null");

    final var u32OutOfRange = U32.of(256);
    assertNull(u32OutOfRange.byteValue(), "U32 with value > 255 should return null for byteValue");
  }

  @Test
  void testU32ByteValueBoundary() {
    final var u32At127 = U32.of(127);
    assertNotNull(u32At127.byteValue(), "U32 with value 127 should return non-null");

    final var u32At128 = U32.of(128);
    assertNull(u32At128.byteValue(), "U32 with value 128 should return null (negative as byte)");
  }

  @Test
  void testU64OfValidValues() {
    // Test boundary values
    final var u64Zero = U64.of(BigInteger.ZERO);
    assertEquals(0L, u64Zero.longValue());
    assertEquals(0, u64Zero.intValue());

    final var u64Max = U64.of(new BigInteger("18446744073709551615")); // 2^64 - 1
    assertEquals(-1L, u64Max.longValue()); // -1 in signed long representation
  }

  @Test
  void testU64OfNegativeValue() {
    assertThrows(
        IllegalArgumentException.class,
        () -> U64.of(BigInteger.valueOf(-1)),
        "U64 should reject negative values");
  }

  @Test
  void testU64OfValueTooLarge() {
    final var tooLarge = new BigInteger("18446744073709551616"); // 2^64
    assertThrows(
        IllegalArgumentException.class, () -> U64.of(tooLarge), "U64 should reject values >= 2^64");
  }

  @ParameterizedTest
  @ValueSource(longs = {0L, 1L, Long.MAX_VALUE, -1L})
  void testU64ValidRange(long value) {
    final var u64 = U64.of(BigInteger.valueOf(value & 0x7FFFFFFFFFFFFFFFL));
    assertNotNull(u64.longValue(), "U64 should accept valid unsigned values");
  }

  @Test
  void testU64RecordProperties() {
    final var u64 = U64.of(BigInteger.valueOf(0x123456789ABCDEFL));
    assertEquals(0x123456789ABCDEFL, u64.value(), "Record should expose value");
  }

  @Test
  void testU64ToString() {
    assertEquals("u64(0)", U64.of(BigInteger.ZERO).toString());
    final var u64Max = U64.of(new BigInteger("18446744073709551615"));
    assertEquals("u64(18446744073709551615)", u64Max.toString());
  }

  @Test
  void testU64Equality() {
    final var u64a = U64.of(BigInteger.valueOf(0x123456789ABCDEFL));
    final var u64b = U64.of(BigInteger.valueOf(0x123456789ABCDEFL));
    final var u64c = U64.of(BigInteger.valueOf(0x123456789ABCDF0L));

    assertEquals(u64a, u64b, "U64 records with same value should be equal");
    assertNotEquals(u64a, u64c, "U64 records with different values should not be equal");
  }

  @Test
  void testU64ImplementsUnsigned() {
    final var u64 = U64.of(BigInteger.valueOf(1000000L));
    assertTrue(u64 instanceof Unsigned, "U64 should implement Unsigned");
    assertNotNull(u64.longValue());
  }

  @Test
  void testU64ByteValueReturnsNullWhenOutOfRange() {
    final var u64InRange = U64.of(BigInteger.valueOf(100));
    assertNotNull(u64InRange.byteValue(), "U64 with value in byte range should return non-null");

    final var u64OutOfRange = U64.of(BigInteger.valueOf(256));
    assertNull(u64OutOfRange.byteValue(), "U64 with value > 255 should return null for byteValue");
  }

  @Test
  void testU64IntValueReturnsNullWhenOutOfRange() {
    final var u64InRange = U64.of(BigInteger.valueOf(0x12345678L));
    assertNotNull(u64InRange.intValue(), "U64 with value in int range should return non-null");

    final var u64OutOfRange = U64.of(BigInteger.valueOf(0x100000000L));
    assertNull(u64OutOfRange.intValue(), "U64 with value > 2^31-1 should return null for intValue");
  }

  @Test
  void testU64ByteAndIntValueBoundaries() {
    // Test byte boundary - byteValue returns non-null only for 0-127
    final var u64At127 = U64.of(BigInteger.valueOf(127));
    assertNotNull(u64At127.byteValue(), "U64 with value 127 should return non-null for byteValue");

    final var u64At128 = U64.of(BigInteger.valueOf(128));
    assertNull(
        u64At128.byteValue(),
        "U64 with value 128 should return null for byteValue (negative as byte)");

    // Test int boundary
    final var u64AtMaxInt = U64.of(BigInteger.valueOf(0x7FFFFFFFL));
    assertNotNull(
        u64AtMaxInt.intValue(), "U64 with value 2^31-1 should return non-null for intValue");

    final var u64AtMaxIntPlus1 = U64.of(BigInteger.valueOf(0x80000000L));
    assertNull(
        u64AtMaxIntPlus1.intValue(),
        "U64 with value 2^31 should return null for intValue (negative as int)");
  }

  @Test
  void testConversionBetweenTypes() {
    final var u8 = U8.of(42);
    final var u32 = U32.of(42L);
    final var u64 = U64.of(BigInteger.valueOf(42));

    assertEquals(u8.intValue(), u32.intValue());
    assertEquals(u32.longValue(), u64.longValue());
    assertEquals(u8.longValue(), u64.longValue());
  }

  @Test
  void testU8WithHighByteValue() {
    // Test that U8 properly handles value 255 as unsigned
    final var u8 = U8.of(255);
    assertEquals((byte) -1, u8.byteValue()); // Signed byte representation
    assertEquals(255, u8.intValue()); // Unsigned int value
    assertEquals(255L, u8.longValue()); // Unsigned long value
  }
}
