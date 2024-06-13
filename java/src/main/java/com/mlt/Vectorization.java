package com.mlt;

// import jdk.incubator.vector.IntVector;
// import jdk.incubator.vector.VectorOperators;
// import jdk.incubator.vector.VectorSpecies;

import java.nio.IntBuffer;
import java.util.ArrayList;

public class Vectorization {

  // static final VectorSpecies<Integer> SPECIES = IntVector.SPECIES_PREFERRED;

  public static void main(String[] args) {
    var l = 1000000;
    var arr = new int[l];
    var arr2 = new int[l];

    for (var k = 0; k < 100; k++) {
      for (var i = 0; i < l; i++) {
        arr[i] = (int) Math.random() * 100;
      }
      var buff = IntBuffer.wrap(arr);

      var start = System.nanoTime();
      for (var i = 0; i < l; i++) {
        arr2[i] = arr[i];
      }
      var end = System.nanoTime();
      System.out.println("Array: " + ((end - start) / 1e6));
      // System.out.println("Array: " + ((end - start)));
      var start2 = System.nanoTime();
      for (var i = 0; i < l; i++) {
        arr2[i] = buff.get(i);
      }
      var end2 = System.nanoTime();
      System.out.println("Buffer: " + ((end2 - start2) / 1e6));
      // System.out.println("Buffer: " + ((end2 - start2)));
    }

    var arrLength = 10000;
    var values = new int[arrLength];
    var runs = new int[arrLength];
    for (int i = 0; i < arrLength; i++) {
      values[i] = (int) (Math.random() * 10);
      runs[i] = (int) (Math.ceil(Math.random() * 15));
    }

    var scalarDiff = new ArrayList<Double>();
    var vectorizedDiff = new ArrayList<Double>();
    for (var j = 0; j < 1000; j++) {
      var scalarResult = new int[arrLength * 10 + 8];
      var start = System.nanoTime();
      decodeRleScalar(scalarResult, runs, values, arrLength);
      var end = System.nanoTime();
      var diff = (end - start) / 1e6;
      if (j > 100) {
        scalarDiff.add(diff);
      }

      var vectorizedResult = new int[arrLength * 10 + 8];
      var start2 = System.nanoTime();
      // decodeRleVectorized(vectorizedResult, runs, values, arrLength);
      var end2 = System.nanoTime();
      var diff2 = (end2 - start2) / 1e6;
      if (j > 100) {
        vectorizedDiff.add(diff2);
      }
      for (var k = 0; k < arrLength; k++) {
        if (vectorizedResult[k] - scalarResult[k] != 0) {
          throw new RuntimeException("Error");
        }
      }
    }

    var scalarSum = scalarDiff.stream().mapToDouble(Double::doubleValue).average().getAsDouble();
    var vectorizedSum =
        vectorizedDiff.stream().mapToDouble(Double::doubleValue).average().getAsDouble();
    System.out.println("Scalar version: " + scalarSum);
    System.out.println("Vectorized version: " + vectorizedSum);
    System.out.println("Improvement: " + (1 - (vectorizedSum / scalarSum)) * 100);
  }

  public static void profileDeltaEncoding() {
    var arrLength = 1000000;
    // var arrLength = 24;
    var arr = new int[arrLength];
    for (int i = 0; i < arrLength; i++) {
      arr[i] = (int) (Math.random() * 10);
      // System.out.println(arr[i]);
    }
    var encodedArr = encodeZigZagDelta(arr);
    var encodedArr2 = encodedArr.clone();

    var scalarDiff = new ArrayList<Double>();
    var vectorizedDiff = new ArrayList<Double>();
    // System.out.println("Number of Lanes: " + SPECIES.length());
    for (var j = 0; j < 250; j++) {
      var start = System.nanoTime();
      // decodeZigZagDeltaScalar2(encodedArr);
      scalarBroadcast(arr, 8);
      var end = System.nanoTime();
      var diff = (end - start) / 1e6;
      if (j > 100) {
        scalarDiff.add(diff);
      }
      /*System.out.println("------------------------------------");
      for(var k = 0; k < arrLength; k++){
          System.out.println(encodedArr[k]);
      }*/
      // System.out.println("Scalar version: " + diff + "-----------------------");

      var start2 = System.nanoTime();
      // decodeZigZagDeltaVectorized(encodedArr2);
      // vectorizedBroadcast(arr);
      var end2 = System.nanoTime();
      var diff2 = (end2 - start2) / 1e6;
      // System.out.println("Vectorized version: " + (end - start) / 1e6);
      if (j > 100) {
        vectorizedDiff.add(diff2);
      }

      /*System.out.println("------------------------------------");
      for(var k = 0; k < arrLength; k++){
          System.out.println(encodedArr2[k]);
      }*/
    }

    var scalarSum = scalarDiff.stream().mapToDouble(Double::doubleValue).average().getAsDouble();
    var vectorizedSum =
        vectorizedDiff.stream().mapToDouble(Double::doubleValue).average().getAsDouble();
    System.out.println("Scalar version: " + scalarSum);
    System.out.println("Vectorized version: " + vectorizedSum);
    System.out.println("Improvement: " + (1 - (vectorizedSum / scalarSum)) * 100);
  }

  /*public static void main(String[] args){
      var arrLength = 100000;
      var arr1 = new int[arrLength];
      var arr2 = new int[arrLength];

      System.out.println(SPECIES.length());
      for(var j = 0; j <20; j++){
          System.out.println("-----------------------------------------");

          for(int i = 0; i < arrLength; i++) {
              arr1[i] = (int)Math.random();
              arr2[i] = (int)Math.random();
          }

          var start = System.nanoTime();
          //addTwoScalarArrays(arr1, arr2);
          scalarBroadcast(arr1, 8);
          var end = System.nanoTime();
          System.out.println("Scalar version: " + (end - start) / 1e6);

          start = System.nanoTime();
          //addTwoVectorsWithMasks(arr1, arr2);
          //addTwoVectorArrays(arr1, arr2);
          vectorizedBroadcast(arr1);
          end = System.nanoTime();
          System.out.println("Vectorized version: " + (end - start) / 1e6);
      }
  }*/

  /*public static void decodeRleVectorized(int[] dst, int[] runlen, int[] values, int runcnt) {
      int pos = 0;
      for (int run = 0; run < runcnt; run++) {
          int count = runlen[run];
          IntVector runVector = IntVector.broadcast(SPECIES, values[run]);
          int i = 0;
          for (; i <= count; i += SPECIES.length()) {
              runVector.intoArray(dst, pos + i);
          }

          pos += count;
      }

      //TODO: reset the overflowed elements
  }*/

  public static void decodeRleScalar(int[] dst, int[] runlen, int[] values, int runcnt) {
    int dstIndex = 0;
    for (int runIndex = 0; runIndex < runcnt; runIndex++) {
      int count = runlen[runIndex];
      int value = values[runIndex];
      for (int i = 0; i < count; i++) {
        dst[dstIndex++] = value;
      }
    }
  }

  public static void decodeZigZagDeltaScalar2(int[] data) {
    var previousElement = 0;
    for (int i = 0; i < data.length; i++) {
      var currentElement = data[i];
      data[i] = previousElement + ((currentElement >> 1) ^ -(currentElement & 1));
      previousElement = data[i];
    }
  }

  /*public static void decodeZigZagDeltaVectorized(int[] data) {
      final int length = data.length;

      var encodedVector = IntVector.fromArray(SPECIES, data, 0);
      var signVector = encodedVector.lanewise(VectorOperators.AND, 1).lanewise(VectorOperators.NEG);
      var decodedVector = encodedVector.lanewise(VectorOperators.ASHR, 1).lanewise(VectorOperators.XOR, signVector);
      decodedVector = decodedVector.add(decodedVector.unslice(1));
      decodedVector = decodedVector.add(decodedVector.unslice(2));
      decodedVector = decodedVector.add(decodedVector.unslice(4));
      decodedVector.intoArray(data, 0);

      int i = 8;
      for (; i < SPECIES.loopBound(length); i += SPECIES.length()) {
          decodedVector = IntVector.fromArray(SPECIES, data, i);
          signVector = decodedVector.lanewise(VectorOperators.AND, 1).lanewise(VectorOperators.NEG);
          decodedVector = decodedVector.lanewise(VectorOperators.ASHR, 1).lanewise(VectorOperators.XOR, signVector);

          decodedVector = decodedVector.add(decodedVector.unslice(1));
          decodedVector = decodedVector.add(decodedVector.unslice(2));
          decodedVector = decodedVector.add(decodedVector.unslice(4));
          var absoluteValue = IntVector.broadcast(SPECIES, data[i - 1]);
          decodedVector = decodedVector.add(absoluteValue);
          decodedVector.intoArray(data, i);
      }
  }*/

  public static int[] scalarBroadcast(int[] arr, int size) {
    var result = new int[arr.length * size];
    var resultCounter = 0;
    for (var i = 0; i < arr.length; i++) {
      for (var j = 0; j < size; j++) {
        result[resultCounter++] = arr[i];
      }
    }
    return result;
  }

  /*public static int[] vectorizedBroadcast(int[] arr){
      int[] result = new int[arr.length * SPECIES.length()];
      int resultCounter = 0;
      var length = arr.length;
      for (int i = 0; i < SPECIES.loopBound(length); i += SPECIES.length()) {
          IntVector vector = IntVector.broadcast(SPECIES, arr[i]);
          vector.intoArray(result, resultCounter);
          resultCounter += SPECIES.length();
      }
      return result;
  }*/

  public static int[] addTwoScalarArrays(int[] arr1, int[] arr2) {
    int[] result = new int[arr1.length];
    for (int i = 0; i < arr1.length; i++) {
      // result[i] = arr1[i] + arr2[i];
      var a = arr1[i] * arr2[i];
      var b = a * arr2[i];
      result[i] = b + a;
    }
    return result;
  }

  /*public static int[] addTwoVectorArrays(int[] arr1, int[] arr2) {
      int[] finalResult = new int[arr1.length];
      int i = 0;
      for (; i < SPECIES.loopBound(arr1.length); i += SPECIES.length()) {
          var v1 = IntVector.fromArray(SPECIES, arr1, i);
          var v2 = IntVector.fromArray(SPECIES, arr2, i);

          IntVector vectorC = v1.mul(v2);
          var vectorD = vectorC.mul(v2);
          var vectorE = vectorD.add(vectorC);

          // Store the result back into the array
          vectorE.intoArray(finalResult, i);
      }

      return finalResult;
  }*/

  /*public static int[] addTwoVectorsWithMasks(int[] arr1, int[] arr2) {
      int[] finalResult = new int[arr1.length];
      int i = 0;
      for (; i < SPECIES.loopBound(arr1.length); i += SPECIES.length()) {
          var mask = SPECIES.indexInRange(i, arr1.length);
          var v1 = IntVector.fromArray(SPECIES, arr1, i, mask);
          var v2 = IntVector.fromArray(SPECIES, arr2, i, mask);
          var result = v1.add(v2, mask);
          result.intoArray(finalResult, i, mask);
      }

      // tail cleanup loop
      for (; i < arr1.length; i++) {
          finalResult[i] = arr1[i] + arr2[i];
      }
      return finalResult;
  }*/

  private static int[] encodeZigZagDelta(int[] originalArray) {
    for (int i = 0; i < originalArray.length; i++) {
      originalArray[i] = encodeZigZag(originalArray[i]);
    }

    return deltaEncode(originalArray);
  }

  private static int[] deltaEncode(int[] originalArray) {
    if (originalArray.length == 0) return originalArray;

    int[] deltaEncodedArray = new int[originalArray.length];
    deltaEncodedArray[0] = originalArray[0]; // first element remains the same

    for (int i = 1; i < originalArray.length; i++) {
      deltaEncodedArray[i] = originalArray[i] - originalArray[i - 1];
    }

    return deltaEncodedArray;
  }

  private static int encodeZigZag(int n) {
    return (n << 1) ^ (n >> 31);
  }
}
