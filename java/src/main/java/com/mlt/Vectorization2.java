package com.mlt;

// import jdk.incubator.vector.IntVector;
// import jdk.incubator.vector.VectorSpecies;

import java.nio.Buffer;
import java.util.BitSet;
import java.util.Optional;

public class Vectorization2 {

  /*static final VectorSpecies<Integer> SPECIES = IntVector.SPECIES_PREFERRED;

  public static void main(String[] args) {
      int[] arrayA = new int[1_000_000];
      int[] arrayB = new int[1_000_000];
      int[] result = new int[1_000_000];

      // Initialize the arrays with some value.
      for (int i = 0; i < arrayA.length; i++) {
          arrayA[i] = i;
          arrayB[i] = i;
      }

      // Warm-up phase
      for (int i = 0; i < 10_000; i++) {
          scalarAdd(arrayA, arrayB, result);
          simdAdd(arrayA, arrayB, result);
      }

      // Scalar benchmark
      long startTime = System.nanoTime();
      scalarAdd(arrayA, arrayB, result);
      long endTime = System.nanoTime();
      long scalarTime = endTime - startTime;
      System.out.println("Scalar Time: " + scalarTime + " ns");

      // SIMD benchmark
      startTime = System.nanoTime();
      simdAdd(arrayA, arrayB, result);
      endTime = System.nanoTime();
      long simdTime = endTime - startTime;
      System.out.println("SIMD Time: " + simdTime + " ns");
  }

  private static void scalarAdd(int[] arrayA, int[] arrayB, int[] result) {
      for (int i = 0; i < arrayA.length; i++) {
          //result[i] = arrayA[i] + arrayB[i];
          //result[i] = arrayA[i] * arrayB[i] * arrayA[i];
          var a = arrayA[i] * arrayB[i];
          var b = a * arrayB[i];
          result[i] = b + a;
      }
  }

  private static void simdAdd(int[] arrayA, int[] arrayB, int[] result) {
      int i = 0;
      int upperBound = SPECIES.loopBound(arrayA.length);
      for (; i < upperBound; i += SPECIES.length()) {
          // Load vectors from the arrays
          IntVector vectorA = IntVector.fromArray(SPECIES, arrayA, i);
          IntVector vectorB = IntVector.fromArray(SPECIES, arrayB, i);

          // Perform the addition
          //IntVector vectorC = vectorA.add(vectorB);
          IntVector vectorC = vectorA.mul(vectorB);
          var vectorD = vectorC.mul(vectorB);
          var vectorE = vectorD.add(vectorC);

          // Store the result back into the array
          vectorE.intoArray(result, i);
      }
      // Handle the remaining elements which couldn't be processed by the vector loop
      for (; i < arrayA.length; i++) {
          result[i] = arrayA[i] + arrayB[i];
      }
  }*/

  public static record FixedSizeVector<T extends Buffer>(Optional<BitSet> presentStream, T data) {}
}
