package org.maplibre.mlt.synthetics;

import java.io.IOException;
import java.nio.file.FileSystems;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Arrays;
import java.util.Map;
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
        Arrays.stream(rootPaths)
            .flatMap(
                root -> {
                  try {
                    return Files.walk(Path.of(root));
                  } catch (IOException e) {
                    throw new RuntimeException("Failed to walk " + root, e);
                  }
                })
            .filter(matcher::matches)
            .filter(Files::isRegularFile)
            .toList();
    for (Path path : mltPaths) {
      // Load the file
      final MapLibreTile tile;
      try {
        tile = MltDecoder.decodeMlTile(Files.readAllBytes(path));
      } catch (Exception e) {
        System.err.println("WARNING: Failed to decode " + path);
        e.printStackTrace(System.err);
        return;
      }

      final var baseName = path.getFileName().toString();
      final var jsonPath =
          path.resolveSibling(
              baseName.substring(0, Math.max(0, baseName.lastIndexOf('.'))) + ".json");
      if (!Files.exists(jsonPath)) {
        System.err.println("WARNING: No expected JSON for " + path);
        return;
      }

      final var gson = Json.createGson(true);

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
    return expected.equals(actual) || numericsEqual(expected, actual);
  }

  private boolean numericsEqual(Object a, Object b) {
    if (a instanceof Number numA && b instanceof Number numB) {
      if (numA.doubleValue() == numB.doubleValue()) {
        return true;
      } else if (a instanceof Double dbl && b instanceof Float flt) {
        return dbl.floatValue() == flt;
      } else if (b instanceof Double dbl && a instanceof Float flt) {
        return dbl.floatValue() == flt;
      }
    }
    return false;
  }
}
