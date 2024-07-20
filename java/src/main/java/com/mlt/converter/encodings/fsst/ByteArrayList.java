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
    ensureCapacity(2);
    buf[length] = (byte) i;
    buf[length + 1] = (byte) j;
    length += 2;
  }

  public void add(int i) {
    ensureCapacity(1);
    buf[length++] = (byte) i;
  }

  public void add(byte[] bytes) {
    if (bytes.length == 1) {
      add(bytes[0]);
    } else if (bytes.length > 1) {
      int n = bytes.length;
      ensureCapacity(n);
      System.arraycopy(bytes, 0, buf, length, n);
      length += n;
    }
  }

  byte[] toArray() {
    return Arrays.copyOf(buf, length);
  }

  private void ensureCapacity(int additions) {
    if (buf.length - length < additions) {
      int grow = Math.max(additions, buf.length >> 1);
      int newSize = buf.length + grow;
      if (newSize < 0) {
        throw new OutOfMemoryError("Cannot add " + additions + " bytes to existing " + buf.length);
      }
      buf = Arrays.copyOf(buf, newSize);
    }
  }
}
