package org.maplibre.mlt.converter.encodings;

import java.nio.ByteBuffer;
import me.lemire.integercompression.Composition;
import me.lemire.integercompression.FastPFOR;
import me.lemire.integercompression.VariableByte;

/// Reusable FastPFOR codec that clients can reuse for many tile encoder calls from a single thread.
/// This fixes a bug where JavaFastPFOR doesn't fully reset its state between calls.
public class FastPforCodec extends Composition {

  public FastPforCodec() {
    // Improve throughput by using heap instead of direct byte buffers (JDK's global Cleaner lock
    // stalls threads when running many encoders in parallel)
    super(
        new FastPFOR() {
          @Override
          protected ByteBuffer makeBuffer(int sizeInBytes) {
            return ByteBuffer.allocate(sizeInBytes);
          }
        },
        new VariableByte() {
          @Override
          protected ByteBuffer makeBuffer(int sizeInBytes) {
            return ByteBuffer.allocate(sizeInBytes);
          }
        });
  }
}
