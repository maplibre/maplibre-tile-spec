package org.maplibre.mlt.converter.encodings;

import static com.mlt.converter.encodings.LinearRegression.*;

import org.maplibre.mlt.TestSettings;
import org.maplibre.mlt.converter.geometry.HilbertCurve;
import org.maplibre.mlt.converter.geometry.Vertex;
import org.maplibre.mlt.converter.mvt.MvtUtils;
import java.io.IOException;
import java.nio.file.Paths;
import java.util.Arrays;
import java.util.Collections;
import java.util.stream.Collectors;
import java.util.stream.DoubleStream;
import java.util.stream.IntStream;
import java.util.stream.Stream;
import org.junit.jupiter.api.Test;

public class LinearRegressionTest {

  @Test
  public void test() throws IOException {
    var tileId = String.format("%s_%s_%s", 5, 16, 20);
    var mvtFilePath = Paths.get(TestSettings.OMT_MVT_PATH, tileId + ".mvt");
    var mvTile = MvtUtils.decodeMvt(mvtFilePath);

    for (var layer : mvTile.layers()) {
      if (layer.name().equals("transportation")) {
        var features = layer.features();
        var geometries = features.stream().map(f -> f.geometry()).collect(Collectors.toList());
        var vertices =
            geometries.stream()
                .map(g -> g.getCoordinates())
                .flatMap(i -> Stream.of(i[0], i[1]))
                .collect(Collectors.toList());

        var minVertexValue =
            Collections.min(
                vertices.stream()
                    .flatMapToDouble(v -> DoubleStream.of(v.getX(), v.getY()))
                    .boxed()
                    .collect(Collectors.toList()));
        var maxVertexValue =
            Collections.max(
                vertices.stream()
                    .flatMapToDouble(v -> DoubleStream.of(v.getX(), v.getY()))
                    .boxed()
                    .collect(Collectors.toList()));
        var hilbertCurve = new HilbertCurve(minVertexValue.intValue(), maxVertexValue.intValue());
        var hilbertIndices =
            vertices.stream()
                .mapToInt(i -> hilbertCurve.encode(new Vertex((int) i.getX(), (int) i.getY())))
                .toArray();
        var sortedHilbertIndices =
            Arrays.stream(hilbertIndices).sorted().boxed().collect(Collectors.toList());
        // sortedHilbertIndices =
        // sortedHilbertIndices.stream().distinct().limit(25).collect(Collectors.toList());
        sortedHilbertIndices =
            sortedHilbertIndices.stream().distinct().collect(Collectors.toList()).subList(82, 99);

        var deltas =
            EncodingUtils.encodeDeltas(sortedHilbertIndices.stream().mapToInt(i -> i).toArray());

        double[] indices = IntStream.range(0, deltas.length).boxed().mapToDouble(i -> i).toArray();
        double alpha = 0.01; // example learning rate
        int iterations = 1000; // example number of iterations
        double[] J = new double[iterations]; // to store cost history
        double[] theta =
            gradientDescent(
                indices,
                sortedHilbertIndices.stream().mapToDouble(i -> i).toArray(),
                alpha,
                iterations,
                J);
        var deltasLinearRegression =
            calculateDeltas(
                indices, sortedHilbertIndices.stream().mapToDouble(i -> i).toArray(), theta);
        var modifiedDeltasLinearRegression =
            Arrays.copyOfRange(deltasLinearRegression, 1, deltasLinearRegression.length);

        var modifiedDeltas = Arrays.copyOfRange(deltas, 1, deltas.length);
        var deltaSum = Arrays.stream(modifiedDeltas).sum();
        var deltaLinearRegressionSum = Arrays.stream(modifiedDeltasLinearRegression).sum();
        System.out.println(
            "delta: " + deltaSum + " linear Regression delta: " + deltaLinearRegressionSum);
        System.out.println(
            "max delta: "
                + Arrays.stream(modifiedDeltas).max().getAsInt()
                + " max linear Regression delta: "
                + Arrays.stream(modifiedDeltasLinearRegression).max().getAsDouble());
      }
    }
  }

  @Test
  public void test2() throws IOException {
    var tileId = String.format("%s_%s_%s", 5, 16, 20);
    var mvtFilePath = Paths.get(TestSettings.OMT_MVT_PATH, tileId + ".mvt");
    var mvTile = MvtUtils.decodeMvt(mvtFilePath);

    for (var layer : mvTile.layers()) {
      if (layer.name().equals("transportation")) {
        var features = layer.features();
        var geometries = features.stream().map(f -> f.geometry()).collect(Collectors.toList());
        var vertices =
            geometries.stream()
                .map(g -> g.getCoordinates())
                .flatMap(i -> Stream.of(i[0], i[1]))
                .collect(Collectors.toList());

        // var xCoordinates = vertices.stream().mapToInt(i ->
        // (int)i.getX()).distinct().sorted().boxed()
        // .collect(Collectors.toList()).subList(82, 99);
        var xCoordinates =
            vertices.stream()
                .mapToInt(i -> (int) i.getX())
                .distinct()
                .boxed()
                .collect(Collectors.toList())
                .subList(81, 104);

        var deltas = EncodingUtils.encodeDeltas(xCoordinates.stream().mapToInt(i -> i).toArray());

        double[] indices = IntStream.range(0, deltas.length).boxed().mapToDouble(i -> i).toArray();
        double alpha = 0.01; // example learning rate
        int iterations = 1000; // example number of iterations
        double[] J = new double[iterations]; // to store cost history
        double[] theta =
            gradientDescent(
                indices, xCoordinates.stream().mapToDouble(i -> i).toArray(), alpha, iterations, J);
        var deltasLinearRegression =
            calculateDeltas(indices, xCoordinates.stream().mapToDouble(i -> i).toArray(), theta);
        var modifiedDeltasLinearRegression =
            Arrays.copyOfRange(deltasLinearRegression, 1, deltasLinearRegression.length);

        var modifiedDeltas = Arrays.copyOfRange(deltas, 1, deltas.length);
        var deltaSum = Arrays.stream(modifiedDeltas).sum();
        var deltaLinearRegressionSum = Arrays.stream(modifiedDeltasLinearRegression).sum();
        System.out.println(
            "delta: " + deltaSum + " linear Regression delta: " + deltaLinearRegressionSum);
        System.out.println(
            "max delta: "
                + Arrays.stream(modifiedDeltas).max().getAsInt()
                + " max linear Regression delta: "
                + Arrays.stream(modifiedDeltasLinearRegression).max().getAsDouble());
      }
    }
  }
}
