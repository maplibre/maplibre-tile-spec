package org.maplibre.mlt.util;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.Optional;
import org.junit.jupiter.api.Test;

class OptionalUtilTest {
  @Test
  void isLessThanBothEmpty() {
    assertFalse(OptionalUtil.isLessThan(Optional.<Integer>empty(), Optional.<Integer>empty()));
  }

  @Test
  void isLessThan_whenLeftEmptyAndRightPresent_returnsFalse() {
    assertFalse(OptionalUtil.isLessThan(Optional.<Integer>empty(), Optional.of(10)));
  }

  @Test
  void isLessThan_whenLeftPresentAndRightEmpty_returnsTrue() {
    assertTrue(OptionalUtil.isLessThan(Optional.of(10), Optional.<Integer>empty()));
  }

  @Test
  void isLessThanLess() {
    assertTrue(OptionalUtil.isLessThan(Optional.of(10), Optional.of(20)));
  }

  @Test
  void isLessThanEqual() {
    assertFalse(OptionalUtil.isLessThan(Optional.of(20), Optional.of(20)));
  }

  @Test
  void isLessThanGreater() {
    assertFalse(OptionalUtil.isLessThan(Optional.of(30), Optional.of(20)));
  }

  @Test
  void isLessThanSupportsComparableTypes() {
    assertTrue(OptionalUtil.isLessThan(Optional.of("a"), Optional.of("b")));
    assertFalse(OptionalUtil.isLessThan(Optional.of("b"), Optional.of("a")));
  }

  @Test
  void isLessThanThrowsWhenOptional1Nulls() {
    assertThrows(NullPointerException.class, () -> OptionalUtil.isLessThan(null, Optional.of(1)));
  }

  @Test
  void isLessThanThrowsWhenOptional2Null() {
    assertThrows(NullPointerException.class, () -> OptionalUtil.isLessThan(Optional.of(1), null));
  }

  @Test
  void mapReturnsMappedValue() {
    final var result = OptionalUtil.map(Optional.of(2), Optional.of(3), Integer::sum);
    assertEquals(Optional.of(5), result);
  }

  @Test
  void mapEmptyWhenOptional1Empty() {
    final var result = OptionalUtil.map(Optional.<Integer>empty(), Optional.of(3), Integer::sum);
    assertEquals(Optional.empty(), result);
  }

  @Test
  void mapEmptyWhenOptional2Empty() {
    final var result = OptionalUtil.map(Optional.of(2), Optional.<Integer>empty(), Integer::sum);
    assertEquals(Optional.empty(), result);
  }

  @Test
  void mapBothEmpty() {
    final var result =
        OptionalUtil.map(Optional.<Integer>empty(), Optional.<Integer>empty(), Integer::sum);
    assertEquals(Optional.empty(), result);
  }

  @Test
  void mapThrowsWhenOptional1Null() {
    assertThrows(
        NullPointerException.class, () -> OptionalUtil.map(null, Optional.of(1), Integer::sum));
  }

  @Test
  void mapThrowsWhenOptional2Null() {
    assertThrows(
        NullPointerException.class, () -> OptionalUtil.map(Optional.of(1), null, Integer::sum));
  }

  @Test
  void mapThrowsWhenFunctionNull() {
    assertThrows(
        NullPointerException.class, () -> OptionalUtil.map(Optional.of(1), Optional.of(2), null));
  }
}
