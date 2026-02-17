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
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.TreeMap;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Future;
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
import org.locationtech.jts.geom.Geometry;
import org.locationtech.jts.io.geojson.GeoJsonWriter;
import org.maplibre.mlt.converter.mvt.MapboxVectorTile;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Layer;
import org.maplibre.mlt.data.MapLibreTile;

public class CliUtil {

  private CliUtil() {}

  public static String printMLT(MapLibreTile mlTile) {
    final var gson =
        new GsonBuilder().setPrettyPrinting().serializeSpecialFloatingPointValues().create();
    return gson.toJson(Map.of("layers", mlTile.layers().stream().map(CliUtil::toJSON).toList()));
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

  public static String printMltGeoJson(MapLibreTile mlTile) {
    final var gson =
        new GsonBuilder().setPrettyPrinting().serializeSpecialFloatingPointValues().create();
    var fc = new TreeMap<String, Object>();
    fc.put("type", "FeatureCollection");
    fc.put(
        "features",
        mlTile.layers().stream()
            .flatMap(
                layer -> layer.features().stream().map(feature -> featureToGeoJson(layer, feature)))
            .toList());
    return gson.toJson(fc);
  }

  private static Map<String, Object> featureToGeoJson(Layer layer, Feature feature) {
    var f = new TreeMap<String, Object>();
    f.put("type", "Feature");
    f.put("id", feature.id());
    var props = getSortedNonNullProperties(feature);
    props.put("_layer", layer.name());
    props.put("_extent", layer.tileExtent());
    f.put("properties", props);
    var geom = feature.geometry();
    f.put("geometry", geom == null ? null : geometryToGeoJson(geom));
    return f;
  }

  // Filters out null values and returns properties sorted by key.
  // Duplicate keys (if any) keep the first value.
  private static TreeMap<String, Object> getSortedNonNullProperties(Feature feature) {
    return feature.properties().entrySet().stream()
        .filter(entry -> entry.getValue() != null)
        .collect(
            Collectors.toMap(Map.Entry::getKey, Map.Entry::getValue, (v1, v2) -> v1, TreeMap::new));
  }

  @SuppressWarnings("unchecked")
  private static Map<String, Object> geometryToGeoJson(Geometry geometry) {
    var writer = new GeoJsonWriter();
    writer.setEncodeCRS(false);
    Map<String, Object> map =
        new GsonBuilder()
            .serializeSpecialFloatingPointValues()
            .create()
            .fromJson(writer.write(geometry), Map.class);
    if (map.containsKey("coordinates")) {
      map.put("coordinates", intifyCoordinates(map.get("coordinates")));
    }
    return map;
  }

  /** Recursively convert whole-number doubles to longs inside a coordinates structure. */
  private static Object intifyCoordinates(Object obj) {
    if (obj instanceof List<?> list) {
      return list.stream().map(CliUtil::intifyCoordinates).toList();
    }
    if (obj instanceof Double d && d == Math.floor(d) && !Double.isInfinite(d)) {
      return d.longValue();
    }
    return obj;
  }

  public static String printMVT(MapboxVectorTile mvTile) {
    final var gson = new GsonBuilder().setPrettyPrinting().create();
    return gson.toJson(Map.of("layers", mvTile.layers().stream().map(CliUtil::toJSON).toList()));
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
        System.err.println(
            "Failed to create GzipCompressorOutputStream, falling back to uncompressed or alternative compression.");
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
}
