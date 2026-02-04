package org.maplibre.mlt.converter.encodings;

import java.io.ByteArrayOutputStream;
import java.io.IOException;

/**
 * Encodes byte data using run-length encoding.
 *
 * <p>The encoding format uses a control byte followed by data:
 *
 * <ul>
 *   <li>Control byte 0x00-0x7F: Run of (control + 3) copies of the next byte
 *   <li>Control byte 0x80-0xFF: (256 - control) literal bytes follow
 * </ul>
 */
public class ByteRleEncoder {
  static final int MIN_REPEAT_SIZE = 3;
  static final int MAX_LITERAL_SIZE = 128;
  static final int MAX_REPEAT_SIZE = 127 + MIN_REPEAT_SIZE;

  private final ByteArrayOutputStream output;
  private final byte[] literals = new byte[MAX_LITERAL_SIZE];
  private int numLiterals = 0;
  private boolean repeat = false;
  private int tailRunLength = 0;

  public ByteRleEncoder() {
    this.output = new ByteArrayOutputStream();
  }

  private void writeValues() throws IOException {
    if (numLiterals != 0) {
      if (repeat) {
        output.write(numLiterals - MIN_REPEAT_SIZE);
        output.write(literals[0] & 0xFF);
      } else {
        output.write(-numLiterals);
        output.write(literals, 0, numLiterals);
      }
      repeat = false;
      tailRunLength = 0;
      numLiterals = 0;
    }
  }

  public void write(byte value) throws IOException {
    if (numLiterals == 0) {
      literals[numLiterals++] = value;
      tailRunLength = 1;
    } else if (repeat) {
      if (value == literals[0]) {
        numLiterals++;
        if (numLiterals == MAX_REPEAT_SIZE) {
          writeValues();
        }
      } else {
        writeValues();
        literals[numLiterals++] = value;
        tailRunLength = 1;
      }
    } else {
      if (value == literals[numLiterals - 1]) {
        tailRunLength++;
      } else {
        tailRunLength = 1;
      }
      if (tailRunLength == MIN_REPEAT_SIZE) {
        if (numLiterals + 1 == MIN_REPEAT_SIZE) {
          repeat = true;
          numLiterals++;
        } else {
          numLiterals -= MIN_REPEAT_SIZE - 1;
          writeValues();
          literals[0] = value;
          repeat = true;
          numLiterals = MIN_REPEAT_SIZE;
        }
      } else {
        literals[numLiterals++] = value;
        if (numLiterals == MAX_LITERAL_SIZE) {
          writeValues();
        }
      }
    }
  }

  public void flush() throws IOException {
    writeValues();
  }

  public byte[] toByteArray() throws IOException {
    flush();
    return output.toByteArray();
  }

  /** Convenience method to encode a byte array. */
  public static byte[] encode(byte[] values) throws IOException {
    ByteRleEncoder encoder = new ByteRleEncoder();
    for (byte value : values) {
      encoder.write(value);
    }
    return encoder.toByteArray();
  }
}
