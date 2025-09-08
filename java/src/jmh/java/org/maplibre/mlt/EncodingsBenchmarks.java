package org.maplibre.mlt;

import java.io.IOException;
import java.nio.ByteBuffer;
import java.nio.IntBuffer;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Random;
import java.util.concurrent.TimeUnit;
import java.util.stream.Collectors;
import me.lemire.integercompression.IntWrapper;
import org.maplibre.mlt.converter.encodings.EncodingUtils;
import org.maplibre.mlt.decoder.vectorized.VectorizedDecodingUtils;
import org.openjdk.jmh.annotations.*;

/*
 * RLE benchmarks
 * Delta benchmarks
 * FastPfor benchmarks
 * C -> P conversion benchmarks
 * */

@State(Scope.Benchmark)
@OutputTimeUnit(TimeUnit.MILLISECONDS)
@BenchmarkMode(Mode.AverageTime)
@Threads(value = 1)
@Warmup(iterations = 1)
@Measurement(iterations = 2)
@Fork(value = 1)
public class EncodingsBenchmarks {
  private int numTotalValues = 0;
  private int numRuns = 5000;
  private int[] data = new int[numRuns * 2];
  private int[] fastPforData = new int[1500000];
  private byte[] fastPforEncodedData;
  private byte[] vectorizedFastPforEncodedData;

  public static void main(String[] args) throws IOException {
    var a = new EncodingsBenchmarks();
    a.setup();
    a.vectorizedFastPfor();
  }

  /*@Setup
  public void setup() throws IOException {
      for (int i = 0; i < numRuns; i++) {
          data[i] = (int) (Math.random() * 10);
          data[i + numRuns] = (int) (Math.ceil(Math.random() * 15));
          numTotalValues += data[i];
      }
  }*/

  @Setup
  public void setup() throws IOException {
    // Path path = Paths.get("./src/jmh/java/com/mlt/data/rle_PartOffsets.csv");
    // Path path = Paths.get("./src/jmh/java/com/mlt/data/rle_class_ratio17.csv");
    Path path = Paths.get("./src/jmh/java/com/mlt/data/rle_id_ratio22_45k.csv");
    var lines = Files.lines(path);
    String data = lines.collect(Collectors.joining("\n"));
    lines.close();

    var parts = data.split(";");
    var v = Arrays.stream(parts).map(Integer::valueOf).mapToInt(i -> i).toArray();
    var encodedValues = EncodingUtils.encodeRle(v);
    var a = new ArrayList<>(encodedValues.getLeft());
    a.addAll(encodedValues.getRight());

    this.data = a.stream().mapToInt(i -> i).toArray();
    this.numTotalValues = v.length;
    this.numRuns = encodedValues.getLeft().size();

    var random = new Random();
    var buffer = allocateAlignedByteBuffer(fastPforData.length * 4, 64);
    var fastPforData2 = buffer.asIntBuffer(); // .array();
    for (var i = 0; i < fastPforData.length; i++) {
      fastPforData[i] = random.nextInt(4096);
      // fastPforData[i] = random.nextInt(100);
      // fastPforData[i] = random.nextInt(500);
      // fastPforData[i] = random.nextInt(120);

      fastPforData2.put(i, fastPforData[i]);

      /*if(i < 1000){
          System.out.println(fastPforData[i]);
      }*/
    }
    // var c = fastPforData2.array();

    fastPforEncodedData = EncodingUtils.encodeFastPfor128(fastPforData, false, false);
    vectorizedFastPforEncodedData =
        EncodingUtils.encodeFastPfor128Vectorized(fastPforData, false, false);

    System.out.println(
        "FastPfor size: " + fastPforEncodedData.length / 1000000d + " ----------------------");
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

  @Benchmark
  public IntBuffer scalarFastPfor() {
    return VectorizedDecodingUtils.decodeFastPfor(
        fastPforEncodedData, fastPforData.length, fastPforEncodedData.length, new IntWrapper(0));
  }

  @Benchmark
  public IntBuffer vectorizedFastPfor() {
    return VectorizedDecodingUtils.decodeFastPforVectorized(
        vectorizedFastPforEncodedData,
        fastPforData.length,
        fastPforEncodedData.length,
        new IntWrapper(0));
  }

  // @Benchmark
  public int[] vectorizedRleDecoding() {
    return VectorizedDecodingUtils.decodeUnsignedRleVectorized(data, numRuns, numTotalValues);
  }

  // @Benchmark
  public int[] scalarRleDecoding() {
    return VectorizedDecodingUtils.decodeUnsignedRLE2(data, numRuns, numTotalValues);
  }

  /*int[] decodeUnsignedRle(int[] values, int[] runs, int numTotalValues) {
      var decodedValues = new int[numTotalValues];
      var offset = 0;
      for (var i = 0; i < runs.length; i++) {
          var runLength = runs[i];
          var value = values[i];
          for (var j = offset; j < offset + runLength; j++) {
              decodedValues[j] = value;
          }
          offset += runLength;
      }

      return decodedValues;
  }

  //private static final VectorSpecies<Integer> SPECIES = IntVector.SPECIES_PREFERRED;
  int[] decodeUnsignedRleVectorized(int[] values, int[] runs, int numTotalValues) {
      var SPECIES = IntVector.SPECIES_PREFERRED;
      var overflow = runs[runs.length - 1] % SPECIES.length();
      var overflowDst = new int[numTotalValues + overflow];
      int pos = 0;
      var numRuns = runs.length;
      for (int run = 0; run < numRuns; run++) {
          int count = runs[run];
          IntVector runVector = IntVector.broadcast(SPECIES, values[run]);
          int i = 0;
          for (; i <= count; i += SPECIES.length()) {
              runVector.intoArray(overflowDst, pos + i);
          }
          pos += count;
      }

      var dst = new int[numTotalValues];
      System.arraycopy(overflowDst, 0, dst, 0, numTotalValues);
      return dst;
  }*/
}
