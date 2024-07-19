package com.mlt.converter.encodings.fsst;

import java.util.Arrays;

/**
 * Lightweight arraylist of primitive bytes as an alaternative to {@link
 * java.io.ByteArrayOutputStream} since it has synchronization overhead.
 */
class ByteArrayList {
  private byte[] buf;
  private int length = 0;

  ByteArrayList() {
    this(32);
  }

  ByteArrayList(int n) {
    buf = new byte[Math.max(32, n)];
  }

  public void add(int i, int j) {
    ensureCapacity(length + 2);
    buf[length] = (byte) i;
    buf[length + 1] = (byte) j;
    length += 2;
  }

  public void add(int i) {
    ensureCapacity(length + 1);
    buf[length++] = (byte) i;
  }

  public void add(byte[] bytes) {
    ensureCapacity(length + bytes.length);
    for (byte b : bytes) {
      buf[length++] = b;
    }
  }

  byte[] toArray() {
    return Arrays.copyOf(buf, length);
  }

  private void ensureCapacity(int size) {
    if (size >= buf.length) {
      buf = Arrays.copyOf(buf, buf.length + (buf.length >> 1));
    }
  }
}
