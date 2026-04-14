package org.maplibre.mlt;

import java.nio.ByteBuffer;
import java.util.concurrent.TimeUnit;
import org.openjdk.jmh.annotations.BenchmarkMode;
import org.openjdk.jmh.annotations.Fork;
import org.openjdk.jmh.annotations.Measurement;
import org.openjdk.jmh.annotations.Mode;
import org.openjdk.jmh.annotations.OutputTimeUnit;
import org.openjdk.jmh.annotations.Scope;
import org.openjdk.jmh.annotations.State;
import org.openjdk.jmh.annotations.Threads;
import org.openjdk.jmh.annotations.Warmup;

@State(Scope.Benchmark)
@OutputTimeUnit(TimeUnit.MILLISECONDS)
@BenchmarkMode(Mode.AverageTime)
@Threads(value = 1)
@Warmup(iterations = 5)
@Measurement(iterations = 5)
@Fork(value = 1)
public class TileProcessingBenchmarks {

  static ByteBuffer allocateAlignedByteBuffer(int capacity, int alignment) {
    // Check if alignment is a power of two
    if (Integer.bitCount(alignment) != 1) {
      throw new IllegalArgumentException("Alignment must be a power of 2");
    }

    // Allocate extra bytes to ensure we can align the buffer
    ByteBuffer buffer = ByteBuffer.allocateDirect(capacity + alignment);

    // Calculate the offset needed to align the buffer
    int offset = alignment - (buffer.position() & (alignment - 1));
    if (offset == alignment) {
      offset = 0; // The buffer is already aligned
    }

    // Create a new buffer with the appropriate position for the alignment
    ByteBuffer alignedBuffer = buffer.position(offset).slice();

    // Set the limit to the original requested capacity
    alignedBuffer.limit(capacity);

    return alignedBuffer;
  }

  class Feature {}

  class FeatureTable {}
}
