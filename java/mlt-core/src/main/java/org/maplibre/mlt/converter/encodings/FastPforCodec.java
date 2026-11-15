package org.maplibre.mlt.converter.encodings;

import java.nio.ByteBuffer;
import me.lemire.integercompression.Composition;
import me.lemire.integercompression.FastPFOR;
import me.lemire.integercompression.IntWrapper;
import me.lemire.integercompression.VariableByte;

/// Reusable FastPFOR codec that clients can reuse for many tile encoder calls from a single thread.
/// This fixes a bug where JavaFastPFOR doesn't fully reset its state between calls.
public class FastPforCodec extends Composition {

  public FastPforCodec() {
    // Improve throughput by using heap instead of direct byte buffers (JDK's global Cleaner lock stalls threads when
    // running many encoders in parallel)
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

  @Override
  public void compress(int[] in, IntWrapper inpos, int inlength, int[] out, IntWrapper outpos) {
    final int outputStart = outpos.get();
    // the stale values are in private arrays so we can only clear them from the result that JavaFastPFOR returns
    super.compress(in, inpos, inlength, out, outpos);
    clearExceptionPadding(out, outputStart, inlength);
  }

  private static void clearExceptionPadding(int[] out, int outputStart, int count) {
    // JavaFastPFOR does not clean up exception bits between runs which could result in different output
    // for the same input. This method clears the bits to ensure it's in the same state for each usage.
    // Decoders only read the bits of the last word that hold exceptions, so zeroing the rest can't
    // change decoded values, and it gives the same bytes a fresh codec writes.
    // TODO: drop once JavaFastPFOR zeroes them itself.
    final int blockValues = count - count % FastPFOR.BLOCK_SIZE;
    final int firstPageStart = outputStart + 1;
    int position = firstPageStart;
    for (int pageStart = 0; pageStart < blockValues; pageStart += FastPFOR.DEFAULT_PAGE_SIZE) {
      final int metadataStart = position + out[position];
      final int metadataBytes = out[metadataStart];
      final int exceptionWidthsPosition = metadataStart + 1 + Math.ceilDiv(metadataBytes, 4);
      final int exceptionWidthsBitmap = out[exceptionWidthsPosition];
      position = exceptionWidthsPosition + 1;
      for (int width = 2; width <= 32; width++) {
        if ((exceptionWidthsBitmap & (1 << (width - 1))) != 0) {
          final int exceptionCount = out[position++];
          final long exceptionBits = (long) exceptionCount * width;
          position += (int) Math.ceilDiv(exceptionBits, 32);
          final int usedBitsInLastWord = (int) (exceptionBits % 32);
          if (usedBitsInLastWord != 0) {
            out[position - 1] &= (1 << usedBitsInLastWord) - 1;
          }
        }
      }
    }
  }
}
