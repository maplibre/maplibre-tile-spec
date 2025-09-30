package org.maplibre.mlt.cli;

import java.util.Comparator;
import java.util.Map;
import java.util.stream.Collectors;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.MapLibreTile;
import org.maplibre.mlt.vector.FeatureTable;

public class CliUtil {

  private CliUtil() {}

  // The method calls below are used to trigger lazy decoding of features, and their return values
  // are intentionally ignored.
  @SuppressWarnings("ResultOfMethodCallIgnored")
  public static void decodeFeatureTables(FeatureTable[] featureTables) {
    for (FeatureTable featureTable : featureTables) {
      for (Feature mltFeature : featureTable) {
        // Trigger decoding of the feature
        mltFeature.id();
        mltFeature.geometry();
        mltFeature.properties();
      }
    }
  }

  public static void printMLT(MapLibreTile mlTile) {
    mlTile
        .layers()
        .forEach(
            layer -> {
              System.out.println(layer.name());
              layer
                  .features()
                  .forEach(
                      feature -> {
                        // Print properties sorted by key and drop those with null values to allow
                        // for direct comparison with MVT output.
                        final var properties =
                            feature.properties().entrySet().stream()
                                .filter(entry -> entry.getValue() != null)
                                .sorted(Comparator.comparing(Map.Entry::getKey))
                                .map(entry -> entry.getKey() + "=" + entry.getValue())
                                .collect(Collectors.joining(", "));
                        System.out.println(
                            "  Feature[id="
                                + feature.id()
                                + ", geometry="
                                + feature.geometry()
                                + ", properties={"
                                + properties
                                + "}]");
                      });
            });
  }
}
