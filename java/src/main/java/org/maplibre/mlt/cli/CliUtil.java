package org.maplibre.mlt.cli;

import com.google.gson.GsonBuilder;
import java.util.Map;
import java.util.TreeMap;
import java.util.stream.Collectors;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Layer;
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

  public static String printMLT(MapLibreTile mlTile) {
    final var gson = new GsonBuilder().setPrettyPrinting().create();
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
}
