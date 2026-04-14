package org.maplibre.mlt.synthetics;

import com.google.gson.GsonBuilder;
import com.google.gson.ToNumberPolicy;
import java.io.IOException;
import java.math.BigDecimal;
import java.nio.file.FileSystems;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.PathMatcher;
import java.util.Arrays;
import java.util.Map;
import java.util.Objects;
import java.util.SequencedCollection;
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.Test;
import org.maplibre.mlt.data.MapLibreTile;
import org.maplibre.mlt.decoder.MltDecoder;
import org.maplibre.mlt.json.Json;

public class SyntheticsTest {
  @Test
  public void checkSynthetics() throws IOException {
    final var matcher = FileSystems.getDefault().getPathMatcher("glob:**/*.mlt");
    final var rootPaths =
        new String[] {"../../test/synthetic/0x01", "../../test/synthetic/0x01-rust"};
    final var mltPaths =
        Arrays.stream(rootPaths).flatMap(root -> findFiles(root, matcher).stream()).toList();
    for (Path path : mltPaths) {
      // Load the file
      final MapLibreTile tile;
      try {
        tile = MltDecoder.decodeMlTile(Files.readAllBytes(path));
      } catch (Exception e) {
        System.err.println("WARNING: Failed to decode " + path);
        e.printStackTrace(System.err);
        continue;
      }

      final var baseName = path.getFileName().toString();
      final var jsonPath =
          path.resolveSibling(
              baseName.substring(0, Math.max(0, baseName.lastIndexOf('.'))) + ".json");
      if (!Files.exists(jsonPath)) {
        System.err.println("WARNING: No expected JSON for " + path);
        continue;
      }

      // Don't let Gson turn Uin64 values into doubles, which loses precision.
      final var gson =
          new GsonBuilder()
              .serializeSpecialFloatingPointValues()
              .setObjectToNumberStrategy(ToNumberPolicy.BIG_DECIMAL)
              .create();

      Object expectedJsonObjects;
      try (var jsonReader = Files.newBufferedReader(jsonPath)) {
        expectedJsonObjects = gson.fromJson(jsonReader, Object.class);
      } catch (Exception e) {
        throw new RuntimeException("Failed to read " + jsonPath, e);
      }

      final var actualJsonObjects = Json.toGeoJsonObjects(tile, gson);

      Assertions.assertTrue(
          compareJsonObjects(expectedJsonObjects, actualJsonObjects),
          "JSON objects do not match for " + path);
    }
  }

  private static SequencedCollection<Path> findFiles(String root, PathMatcher matcher) {
    try (var paths = Files.walk(Path.of(root))) {
      return paths.filter(matcher::matches).filter(Files::isRegularFile).toList();
    } catch (IOException e) {
      throw new RuntimeException("Failed to walk " + root, e);
    }
  }

  private boolean compareJsonObjects(Object expected, Object actual) {
    if (expected instanceof Map<?, ?> expectedMap && actual instanceof Map<?, ?> actualMap) {
      if (expectedMap.size() != actualMap.size()) {
        return false;
      }
      for (var entry : expectedMap.entrySet()) {
        final var key = entry.getKey();
        if (!actualMap.containsKey(key)) {
          return false;
        }
        if (!compareJsonObjects(entry.getValue(), actualMap.get(key))) {
          return false;
        }
      }
      return true;
    } else if (expected instanceof Iterable<?> expectedIterable
        && actual instanceof Iterable<?> actualIterable) {
      final var expectedIterator = expectedIterable.iterator();
      final var actualIterator = actualIterable.iterator();
      while (expectedIterator.hasNext() && actualIterator.hasNext()) {
        if (!compareJsonObjects(expectedIterator.next(), actualIterator.next())) {
          return false;
        }
      }
      return !expectedIterator.hasNext() && !actualIterator.hasNext();
    }
    return Objects.equals(expected, actual) || numericsEqual(expected, actual);
  }

  private boolean numericsEqual(Object a, Object b) {
    if (a instanceof Number numA && b instanceof Number numB) {
      if (numA.doubleValue() == numB.doubleValue()) {
        return true;
      } else if (compareFloats(numA, numB) || compareFloats(numB, numA)) {
        return true;
      } else if (a instanceof BigDecimal || b instanceof BigDecimal) {
        return compareDecimals(a, b) || compareDecimals(b, a);
      }
    }
    return false;
  }

  private boolean compareFloats(Number a, Number b) {
    if (a instanceof Double dbl && b instanceof Float flt) {
      return dbl.floatValue() == flt;
    }
    return false;
  }

  /// Compare values loaded as BigDecimal, with
  /// particular care for unsigned values loaded as, e.g., -1
  private boolean compareDecimals(Object a, Object b) {
    if (a instanceof BigDecimal aDec) {
      if (b instanceof BigDecimal bDec) {
        return aDec.compareTo(bDec) == 0;
      } else if (b instanceof Float fltB) {
        return aDec.floatValue() == fltB || aDec.compareTo(BigDecimal.valueOf(fltB)) == 0;
      } else if (b instanceof Number numB) {
        return numB.intValue() == aDec.intValue()
            || numB.longValue() == aDec.longValue()
            || aDec.compareTo(BigDecimal.valueOf(numB.longValue())) == 0
            || aDec.compareTo(BigDecimal.valueOf(numB.doubleValue())) == 0;
      }
    }
    return false;
  }
}
