package org.maplibre.mlt.cli;

import com.google.gson.GsonBuilder;
import jakarta.annotation.Nullable;
import java.io.BufferedInputStream;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.HashSet;
import java.util.Map;
import java.util.Objects;
import java.util.Set;
import java.util.TreeMap;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Future;
import java.util.concurrent.TimeUnit;
import java.util.function.Predicate;
import java.util.function.Supplier;
import java.util.stream.Collectors;
import java.util.zip.Inflater;
import java.util.zip.InflaterInputStream;
import org.apache.commons.compress.compressors.deflate.DeflateCompressorInputStream;
import org.apache.commons.compress.compressors.deflate.DeflateCompressorOutputStream;
import org.apache.commons.compress.compressors.deflate.DeflateParameters;
import org.apache.commons.compress.compressors.gzip.GzipCompressorInputStream;
import org.apache.commons.compress.compressors.gzip.GzipCompressorOutputStream;
import org.apache.commons.compress.compressors.gzip.GzipParameters;
import org.jetbrains.annotations.NotNull;
import org.maplibre.mlt.converter.ConversionConfig;
import org.maplibre.mlt.converter.mvt.MapboxVectorTile;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Layer;
import org.maplibre.mlt.data.MapLibreTile;

public class CliUtil {

  private CliUtil() {}

  public static String printMLT(MapLibreTile mlTile) {
    final var gson = new GsonBuilder().setPrettyPrinting().create();
    return gson.toJson(Map.of("layers", mlTile.layers().stream().map(CliUtil::toJSON).toList()));
  }

  public static String printMVT(MapboxVectorTile mvTile) {
    final var gson = new GsonBuilder().setPrettyPrinting().create();
    return gson.toJson(Map.of("layers", mvTile.layers().stream().map(CliUtil::toJSON).toList()));
  }

  private static Map<String, Object> toJSON(Layer layer) {
    var map = new TreeMap<String, Object>();
    map.put("name", layer.name());
    map.put("extent", layer.tileExtent());
    map.put("features", layer.features().stream().map(CliUtil::toJSON).toList());
    return map;
  }

  private static Map<String, Object> toJSON(Feature feature) {
    var map = new TreeMap<String, Object>();
    map.put("id", feature.id());
    map.put("geometry", feature.geometry().toString());
    // Print properties sorted by key and drop those with null
    // values to facilitate direct comparison with MVT output.
    map.put(
        "properties",
        feature.properties().entrySet().stream()
            .filter(entry -> entry.getValue() != null)
            .collect(
                Collectors.toMap(
                    Map.Entry::getKey, Map.Entry::getValue, (v1, v2) -> v1, TreeMap::new)));
    return map;
  }

  ///  Execute the given task either directly or on the given thread pool
  public static void runTask(@Nullable ExecutorService threadPool, @NotNull Runnable task) {
    if (threadPool != null) {
      threadPool.submit(() -> task.run());
    } else {
      task.run();
    }
  }

  ///  Execute the given task either directly or on the given thread pool
  public static <T> Future<T> runTask(
      @Nullable ExecutorService threadPool, @NotNull Supplier<T> task) {
    return (threadPool != null)
        ? threadPool.submit(() -> task.get())
        : CompletableFuture.completedFuture(task.get());
  }

  ///  Join the given thread pool if it is not null and optionally close it before joining
  public static void joinThreadPool(@Nullable ExecutorService threadPool, boolean close)
      throws InterruptedException {
    if (threadPool != null) {
      if (close) {
        threadPool.close();
      }
      threadPool.awaitTermination(Long.MAX_VALUE, TimeUnit.SECONDS);

    }
  }

  public static byte[] decompress(InputStream srcStream) throws IOException {
    try {
      InputStream decompressInputStream = null;
      // Check for common compression formats by looking at the header bytes
      // Buffered stream is not closed here because it would also close the underlying stream
      final var readStream = new BufferedInputStream(srcStream);
      if (readStream.available() > 3) {
        readStream.mark(4);
        final var header = readStream.readNBytes(4);
        readStream.reset();

        if (DeflateCompressorInputStream.matches(header, header.length)) {
          // deflate with zlib header
          final var inflater = new Inflater(/* nowrap= */ false);
          decompressInputStream = new InflaterInputStream(readStream, inflater);
        } else if (header[0] == 0x1f && header[1] == (byte) 0x8b) {
          // TODO: why doesn't GZIPInputStream work here?
          // decompressInputStream = new GZIPInputStream(readStream);
          decompressInputStream = new GzipCompressorInputStream(readStream);
        }
      }

      if (decompressInputStream != null) {
        try (final var outputStream = new ByteArrayOutputStream()) {
          decompressInputStream.transferTo(outputStream);
          return outputStream.toByteArray();
        }
      }
    } catch (IndexOutOfBoundsException | IOException ex) {
      System.err.printf("Failed to decompress data: %s%n", ex.getMessage());
    }

    return srcStream.readAllBytes();
  }

  public static OutputStream compressStream(OutputStream src, @NotNull String compressionType) {
    if (Objects.equals(compressionType, "gzip")) {
      try {
        var parameters = new GzipParameters();
        parameters.setCompressionLevel(9);
        return new GzipCompressorOutputStream(src, parameters);
      } catch (IOException ex) {
        System.err.println("Failed to create GzipCompressorOutputStream, falling back to uncompressed or alternative compression.");
        ex.printStackTrace(System.err);
      }
    }
    if (Objects.equals(compressionType, "deflate")) {
      var parameters = new DeflateParameters();
      parameters.setCompressionLevel(9);
      parameters.setWithZlibHeader(false);
      return new DeflateCompressorOutputStream(src);
    }
    return src;
  }

  /// Returns the values that are in set a but not in set b
  static <T> Set<T> getAsymmetricSetDiff(Set<T> a, Set<T> b) {
    Set<T> diff = new HashSet<>(a);
    diff.removeAll(b);
    return diff;
  }

  static void createDir(Path path) {
    if (!Files.exists(path)) {
      try {
        Files.createDirectories(path);
      } catch (IOException ex) {
        System.err.println("Failed to create directory: " + path);
        ex.printStackTrace(System.err);
      }
    }
  }

  ///  Compare the content of the given MLT and MVT tiles and throw a RuntimeException if they
  // differ
  public static void compare(
      MapLibreTile mlTile,
      MapboxVectorTile mvTile,
      boolean compareGeom,
      boolean compareProp,
      ConversionConfig config) {
    final Predicate<Layer> testFilter =
        (Layer x) ->
            (config.getLayerFilterPattern() == null)
                || (config.getLayerFilterPattern().matcher(x.name()).matches()
                    ^ config.getLayerFilterInvert());
    final var mvtLayers =
        mvTile.layers().stream().filter(x -> !x.features().isEmpty()).filter(testFilter).toList();
    final var mltLayers = mlTile.layers();
    if (mltLayers.size() != mvtLayers.size()) {
      final var mvtNames = mvtLayers.stream().map(Layer::name).collect(Collectors.joining(", "));
      final var mltNames = mltLayers.stream().map(Layer::name).collect(Collectors.joining(", "));
      throw new RuntimeException(
          "Number of layers in MLT and MVT tiles do not match:\nMVT:\n"
              + mvtNames
              + "\nMLT:\n"
              + mltNames);
    }
    for (var i = 0; i < mvtLayers.size(); i++) {
      final var mltLayer = mltLayers.get(i);
      final var mvtLayer = mvtLayers.get(i);
      final var mltFeatures = mltLayer.features();
      final var mvtFeatures = mvtLayer.features();
      if (!mltLayer.name().equals(mvtLayer.name())) {
        throw new RuntimeException(
            "Layer index "
                + i
                + " of MVT and MLT tile differ: '"
                + mvtLayer.name()
                + "' != '"
                + mltLayer.name()
                + "'");
      }
      if (mltFeatures.size() != mvtFeatures.size()) {
        throw new RuntimeException(
            "Number of features in MLT and MVT layer '"
                + mvtLayer.name()
                + "' do not match: "
                + mltFeatures.size()
                + " != "
                + mvtFeatures.size());
      }
      for (var j = 0; j < mvtFeatures.size(); j++) {
        final var mvtFeature = mvtFeatures.get(j);
        // Expect features to be written in the same order
        final var mltFeature = mltFeatures.get(j);
        if (mvtFeature.id() != mltFeature.id()) {
          throw new RuntimeException(
              "Feature IDs for index "
                  + j
                  + " in layer '"
                  + mvtLayer.name()
                  + "' do not match: "
                  + mvtFeature.id()
                  + " != "
                  + mltFeature.id());
        }
        if (compareGeom) {
          final var mltGeometry = mltFeature.geometry();
          final var mltGeomValid = mltGeometry.isValid();
          final var mvtGeometry = mvtFeature.geometry();
          final var mvtGeomValid = mvtGeometry.isValid();
          if (mltGeomValid != mvtGeomValid) {
            throw new RuntimeException(
                "Geometry validity in MLT and MVT layers do not match for feature index "
                    + j
                    + " in layer '"
                    + mvtLayer.name()
                    + "': \nMVT:\n"
                    + mvtGeomValid
                    + " : "
                    + mvtGeometry
                    + "\nMLT:\n"
                    + mltGeomValid
                    + " : "
                    + mltGeometry);
          }

          if (mvtGeomValid && !mltGeometry.equals(mvtGeometry)) {
            throw new RuntimeException(
                "Geometries in MLT and MVT layers do not match for feature index "
                    + j
                    + " in layer '"
                    + mvtLayer.name()
                    + "': \nMVT:\n"
                    + mvtGeometry
                    + "\nMLT:\n"
                    + mltGeometry
                    + "\nDifference:\n"
                    + mvtGeometry.difference(mltGeometry));
          }
        }
        if (compareProp) {
          final var mltProperties = mltFeature.properties();
          final var mvtProperties = mvtFeature.properties();
          final var mvtPropertyKeys = mvtProperties.keySet();
          final var nonNullMLTKeys =
              mltProperties.entrySet().stream()
                  .filter(entry -> entry.getValue() != null)
                  .map(Map.Entry::getKey)
                  .collect(Collectors.toUnmodifiableSet());
          // compare keys
          if (!mvtPropertyKeys.equals(nonNullMLTKeys)) {
            final var mvtKeys = getAsymmetricSetDiff(mvtPropertyKeys, nonNullMLTKeys);
            final var mvtKeyStr = mvtKeys.isEmpty() ? "(none)" : String.join(", ", mvtKeys);
            final var mltKeys = getAsymmetricSetDiff(nonNullMLTKeys, mvtPropertyKeys);
            final var mltKeyStr = mltKeys.isEmpty() ? "(none)" : String.join(", ", mltKeys);
            throw new RuntimeException(
                "Property keys in MLT and MVT feature index "
                    + j
                    + " in layer '"
                    + mvtLayer.name()
                    + "' do not match:\nOnly in MVT: "
                    + mvtKeyStr
                    + "\nOnly in MLT: "
                    + mltKeyStr);
          }
          // compare values
          final var unequalKeys =
              mvtProperties.keySet().stream()
                  .filter(
                      key -> !propertyValuesEqual(mvtProperties.get(key), mltProperties.get(key)))
                  .toList();
          if (!unequalKeys.isEmpty()) {
            final var unequalValues =
                unequalKeys.stream()
                    .map(
                        key ->
                            "  "
                                + key
                                + ": mvt="
                                + mvtProperties.get(key)
                                + ", mlt="
                                + mltProperties.get(key)
                                + "\n")
                    .collect(Collectors.joining());
            throw new RuntimeException(
                "Property values in MLT and MVT feature index "
                    + j
                    + " in layer '"
                    + mvtLayer.name()
                    + "' do not match: \n"
                    + unequalValues);
          }
        }
      }
    }
  }

  private static boolean propertyValuesEqual(Object a, Object b) {
    // Try simple equality
    if (Objects.equals(a, b)) {
      return true;
    }
    if (a instanceof Double && b instanceof Float) {
      // We currently encode doubles as floats
      return ((Double) a).floatValue() == (Float) b;
    }
    // Allow for, e.g., int32 and int64 representations of the same number by comparing strings
    return a.toString().equals(b.toString());
  }
}
