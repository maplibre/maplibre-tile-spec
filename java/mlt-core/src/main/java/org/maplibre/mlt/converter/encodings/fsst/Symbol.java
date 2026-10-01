package org.maplibre.mlt.converter.encodings.fsst;

import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;

/**
 * A candidate symbol used when building the FSST table.
 *
 * <p>The first pass of FSST encoding builds single-byte symbols found in the data to encode, and
 * each subsequent pass combines those symbols into larger ones.
 */
class Symbol implements Comparable<Symbol> {
  private final byte[] bytes;
  // performance optimizations: precompute hashcode and store length to save an extra dereference
  private final int length;
  private final int hashcode;

  private Symbol(byte[] bytes, int hashcode) {
    this.bytes = bytes;
    this.length = bytes.length;
    this.hashcode = hashcode;
  }

  public static Symbol of(int b) {
    return new Symbol(new byte[] {(byte) b}, 31 + (byte) b);
  }

  public static Symbol concat(Symbol a, Symbol b) {
    int len = Math.min(SymbolTableBuilder.MAX_SYMBOL_LENGTH, a.length + b.length);
    byte[] result = new byte[len];
    System.arraycopy(a.bytes, 0, result, 0, a.length);
    int todo = len - a.length;
    if (todo > 0) System.arraycopy(b.bytes, 0, result, a.length, todo);
    int hash = a.hashcode;
    for (byte bb : b.bytes) {
      hash = 31 * hash + bb;
    }
    return new Symbol(result, hash);
  }

  @Override
  public boolean equals(Object o) {
    return (this == o) || (o instanceof Symbol other && Arrays.equals(bytes, other.bytes));
  }

  @Override
  public int hashCode() {
    return hashcode;
  }

  public int length() {
    return length;
  }

  public byte[] bytes() {
    return bytes;
  }

  @Override
  public int compareTo(Symbol o) {
    // sort symbols lexicographically except longer symbols are before symbols that are their prefix
    if (this == o) return 0;
    var a = bytes;
    var b = o.bytes;

    int i = Arrays.mismatch(a, b);
    if (i >= 0 && i < a.length && i < b.length) {
      return Byte.compareUnsigned(a[i], b[i]);
    }

    // only difference from Arrays.compareUnsigned - when one is a prefix
    // of the other, sort the longest one first
    return b.length - a.length;
  }

  public int first() {
    return bytes[0] & 0xFF;
  }

  /**
   * Returns true if this symbol appears at index {@code offset} in {@code text}, skipping the first
   * {@code ignore} bytes if we already know they match.
   */
  public boolean match(ByteBuffer text, int offset, int ignore) {
    if (text.capacity() < offset + length) {
      return false;
    }
    // optimization: when symbols are indexed by first 1 or 2 bytes, we already know those bytes
    // match so don't need to check them
    for (int i = ignore; i < length; i++) {
      if (text.get(offset + i) != bytes[i]) {
        return false;
      }
    }
    return true;
  }

  @Override
  public String toString() {
    return "Symbol[" + new String(bytes, StandardCharsets.UTF_8) + "]";
  }
}
