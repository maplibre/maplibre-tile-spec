package org.maplibre.mlt.decoder;

import java.nio.ByteBuffer;

/**
 * Decodes byte run-length encoded data.
 *
 * <p>The encoding format uses a control byte followed by data:
 *
 * <ul>
 *   <li>Control byte 0x00-0x7F: Run of (control + 3) copies of the next byte
 *   <li>Control byte 0x80-0xFF: (256 - control) literal bytes follow
 * </ul>
 */
public class ByteRleDecoder {
  private static final int MIN_REPEAT_SIZE = 3;

  private final ByteBuffer buffer;
  private final byte[] literals = new byte[128];
  private int numLiterals = 0;
  private int used = 0;
  private boolean repeat = false;

  public ByteRleDecoder(byte[] data, int offset, int length) {
    this.buffer = ByteBuffer.wrap(data, offset, length);
  }

  private void readValues() {
    int control = buffer.get() & 0xFF;

    if (control < 0x80) {
      // Run: control + MIN_REPEAT_SIZE copies of the next byte
      repeat = true;
      numLiterals = control + MIN_REPEAT_SIZE;
      literals[0] = buffer.get();
    } else {
      // Literals: 256 - control literal bytes
      repeat = false;
      numLiterals = 256 - control;
      buffer.get(literals, 0, numLiterals);
    }
    used = 0;
  }

  public byte next() {
    if (used == numLiterals) {
      readValues();
    }
    if (repeat) {
      used++;
      return literals[0];
    } else {
      return literals[used++];
    }
  }
}
