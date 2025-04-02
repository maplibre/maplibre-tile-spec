package com.mlt.converter.encodings;

import java.util.Arrays;

// Source: https://github.com/yhliu918/Learn-to-Compress
public class LinearRegression {

  public static double[] calculateDeltas(double[] indices, double[] actualValues, double[] theta) {
    var predictions = calculatePredictions(indices, theta);

    var deltas = new double[actualValues.length];
    for (var i = 0; i < actualValues.length; i++) {
      deltas[i] = Math.abs(predictions[i] - actualValues[i]);
    }

    return deltas;
  }

  public static double[] calculatePredictions(double[] x, double[] theta) {
    int m = x.length;
    double[] predictions = new double[m];
    for (int i = 0; i < m; ++i) {
      predictions[i] = h(x[i], theta);
    }
    return predictions;
  }

  private static double h(double x, double[] theta) {
    return theta[0] + theta[1] * x;
  }

  public static double computeCost(double[] x, double[] y, double[] theta) {
    int m = x.length;
    double[] predictions = calculatePredictions(x, theta);
    double[] diff = arrayDiff(predictions, y);
    double[] sqErrors = arrayPow(diff, 2);
    return (1.0 / (2 * m)) * arraySum(sqErrors);
  }

  public static double[] gradientDescent(
      double[] x, double[] y, double alpha, int iters, double[] J) {
    int m = x.length;
    // int m = 1;
    double[] theta = new double[2];
    theta[1] = (y[m - 1] - y[0]) / (x[m - 1] - x[0]);
    theta[0] = y[0] - theta[1] * x[0];
    for (int i = 0; i < iters; ++i) {
      double[] predictions = calculatePredictions(x, theta);
      double[] diff = arrayDiff(predictions, y);
      double[] errorsX1 = diff;
      double[] errorsX2 = arrayMultiplication(diff, x);
      theta[0] -= alpha * (1.0 / m) * arraySum(errorsX1);
      theta[1] -= alpha * (1.0 / m) * arraySum(errorsX2);
      J[i] = computeCost(x, y, theta);
    }
    return theta;
  }

  public static double[] arrayDiff(double[] arr1, double[] arr2) {
    int len = arr1.length;
    double[] arr = new double[len];
    for (int i = 0; i < len; ++i) {
      arr[i] = arr1[i] - arr2[i];
    }
    return arr;
  }

  public static double[] arrayPow(double[] arr, int power) {
    int len = arr.length;
    double[] arr2 = new double[len];
    for (int i = 0; i < len; ++i) {
      arr2[i] = Math.pow(arr[i], power);
    }
    return arr2;
  }

  public static double[] arrayMultiplication(double[] arr1, double[] arr2) {
    int len = arr1.length;
    double[] arr = new double[len];
    for (int i = 0; i < len; ++i) {
      arr[i] = arr1[i] * arr2[i];
    }
    return arr;
  }

  public static double arraySum(double[] arr) {
    return Arrays.stream(arr).sum();
  }
}
