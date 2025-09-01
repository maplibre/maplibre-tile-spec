package org.maplibre.mlt;

import java.nio.ByteBuffer;
import java.util.concurrent.TimeUnit;
import org.openjdk.jmh.annotations.*;

@State(Scope.Benchmark)
@OutputTimeUnit(TimeUnit.MILLISECONDS)
@BenchmarkMode(Mode.AverageTime)
@Threads(value = 1)
@Warmup(iterations = 5)
@Measurement(iterations = 5)
@Fork(value = 1)
public class TileProcessingBenchmarks {

  @Benchmark
  public com.mlt.vector.FeatureTable[] filterVectorized() {
    /* Filter
     * ["all", ["==", "$type", "LineString"], ["in", "class", "pier"]],
     * -> class = pier
     * -> type == LineString -> DictionaryVector only compare int index
     *
     * ["==", "$type", "LineString"], ["==", "brunnel", "tunnel"], ["==", "class", "minor_road"]
     * -> LineString -> ConstVector
     *
     *
     * ["==", "$type", "LineString"], ["==", "brunnel", "tunnel"], ["in", "class", "primary",
     * "secondary", "tertiary", "trunk"]
     * -> LineString -> ConstIntVector -> compare int
     *   -> only 1 int compared to 70k ints
     *   -> use vectorized comparison
     * -> brunnel -> DictionaryStringVector -> get index for literal compare int
     *
     * -> class -> DictionaryStringVector -> get indices for literals compare ints
     * */

    /*var chunk1 = IntVector.fromArray(SPECIES, array1, i);
    var chunk2 = IntVector.fromArray(SPECIES, array2, i);*/

    return null;
  }

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
