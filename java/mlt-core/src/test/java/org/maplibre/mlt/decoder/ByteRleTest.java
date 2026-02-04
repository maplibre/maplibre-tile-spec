package org.maplibre.mlt.decoder;

import static org.junit.jupiter.api.Assertions.*;

import java.io.IOException;
import me.lemire.integercompression.IntWrapper;
import org.junit.jupiter.api.Test;
import org.maplibre.mlt.converter.encodings.ByteRleEncoder;

public class ByteRleTest {

  @Test
  public void testEncodeDecodeRun() throws IOException {
    // Test encoding a run of identical bytes
    byte[] values = new byte[] {1, 1, 1, 1, 1, 1, 1};

    byte[] encoded = ByteRleEncoder.encode(values);
    assertNotNull(encoded);

    // Decode
    ByteRleDecoder decoder = new ByteRleDecoder(encoded, 0, encoded.length);
    byte[] decoded = new byte[values.length];
    for (int i = 0; i < values.length; i++) {
      decoded[i] = decoder.next();
    }

    assertArrayEquals(values, decoded);
  }

  @Test
  public void testEncodeDecodeLiterals() throws IOException {
    // Test encoding literal bytes (no runs)
    byte[] values = new byte[] {1, 2, 3, 4, 5, 6, 7};

    byte[] encoded = ByteRleEncoder.encode(values);
    assertNotNull(encoded);

    // Decode
    ByteRleDecoder decoder = new ByteRleDecoder(encoded, 0, encoded.length);
    byte[] decoded = new byte[values.length];
    for (int i = 0; i < values.length; i++) {
      decoded[i] = decoder.next();
    }

    assertArrayEquals(values, decoded);
  }

  @Test
  public void testEncodeDecodeMixed() throws IOException {
    // Test encoding a mix of runs and literals
    byte[] values = new byte[] {1, 1, 1, 2, 3, 4, 5, 5, 5, 5};

    byte[] encoded = ByteRleEncoder.encode(values);
    assertNotNull(encoded);

    // Decode
    ByteRleDecoder decoder = new ByteRleDecoder(encoded, 0, encoded.length);
    byte[] decoded = new byte[values.length];
    for (int i = 0; i < values.length; i++) {
      decoded[i] = decoder.next();
    }

    assertArrayEquals(values, decoded);
  }

  @Test
  public void testEncodeDecodeLongRun() throws IOException {
    // Test encoding a long run (exceeding minimum repeat size)
    byte[] values = new byte[150];
    for (int i = 0; i < values.length; i++) {
      values[i] = 42;
    }

    byte[] encoded = ByteRleEncoder.encode(values);
    assertNotNull(encoded);
    // Should be compressed significantly
    assertTrue(encoded.length < values.length);

    // Decode
    ByteRleDecoder decoder = new ByteRleDecoder(encoded, 0, encoded.length);
    byte[] decoded = new byte[values.length];
    for (int i = 0; i < values.length; i++) {
      decoded[i] = decoder.next();
    }

    assertArrayEquals(values, decoded);
  }

  @Test
  public void testEncodeSingleByte() throws IOException {
    byte[] values = new byte[] {7};

    byte[] encoded = ByteRleEncoder.encode(values);
    assertNotNull(encoded);

    // Decode
    ByteRleDecoder decoder = new ByteRleDecoder(encoded, 0, encoded.length);
    assertEquals(7, decoder.next());
  }

  @Test
  public void testEncodeEmpty() throws IOException {
    byte[] values = new byte[0];

    byte[] encoded = ByteRleEncoder.encode(values);
    assertNotNull(encoded);
    assertEquals(0, encoded.length);
  }

  @Test
  public void testEncoderFlush() throws IOException {
    ByteRleEncoder encoder = new ByteRleEncoder();
    encoder.write((byte) 1);
    encoder.write((byte) 2);
    encoder.write((byte) 3);

    byte[] encoded = encoder.toByteArray();
    assertNotNull(encoded);

    ByteRleDecoder decoder = new ByteRleDecoder(encoded, 0, encoded.length);
    assertEquals(1, decoder.next());
    assertEquals(2, decoder.next());
    assertEquals(3, decoder.next());
  }

  @Test
  public void testCompatibilityWithDecodingUtils() throws IOException {
    // Test that our implementation works with the existing DecodingUtils
    byte[] values = new byte[] {5, 5, 5, 5, 5};

    byte[] encoded = ByteRleEncoder.encode(values);

    IntWrapper pos = new IntWrapper(0);
    byte[] decoded = DecodingUtils.decodeByteRle(encoded, values.length, encoded.length, pos);

    assertArrayEquals(values, decoded);
    assertEquals(encoded.length, pos.get());
  }

  @Test
  public void testMinimalRun() throws IOException {
    // Test the minimum run size (3 bytes)
    byte[] values = new byte[] {7, 7, 7};

    byte[] encoded = ByteRleEncoder.encode(values);
    assertNotNull(encoded);

    ByteRleDecoder decoder = new ByteRleDecoder(encoded, 0, encoded.length);
    assertEquals(7, decoder.next());
    assertEquals(7, decoder.next());
    assertEquals(7, decoder.next());
  }

  @Test
  public void testMaxLiteralSize() throws IOException {
    // Test encoding maximum literal size (128 bytes)
    byte[] values = new byte[128];
    for (int i = 0; i < values.length; i++) {
      values[i] = (byte) (i % 256);
    }

    byte[] encoded = ByteRleEncoder.encode(values);
    assertNotNull(encoded);

    ByteRleDecoder decoder = new ByteRleDecoder(encoded, 0, encoded.length);
    byte[] decoded = new byte[values.length];
    for (int i = 0; i < values.length; i++) {
      decoded[i] = decoder.next();
    }

    assertArrayEquals(values, decoded);
  }
}
