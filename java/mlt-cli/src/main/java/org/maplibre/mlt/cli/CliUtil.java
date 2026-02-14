package org.maplibre.mlt.cli;

import com.google.gson.GsonBuilder;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import java.util.stream.Collectors;
import org.locationtech.jts.geom.Geometry;
import org.locationtech.jts.io.geojson.GeoJsonWriter;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Layer;
import org.maplibre.mlt.data.MapLibreTile;

public class CliUtil {

  private CliUtil() {}

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
    if (feature.hasId()) {
      map.put("id", feature.id());
    }
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
    final var gson = new GsonBuilder().setPrettyPrinting().create();
    var fc = new TreeMap<String, Object>();
    fc.put("type", "FeatureCollection");
    fc.put(
        "features",
        mlTile.layers().stream()
            .flatMap(
                layer ->
                    layer.features().stream()
                        .map(feature -> featureToGeoJson(layer.name(), feature)))
            .toList());
    return gson.toJson(fc);
  }

  private static Map<String, Object> featureToGeoJson(String layerName, Feature feature) {
    var f = new TreeMap<String, Object>();
    f.put("type", "Feature");
    if (feature.hasId()) {
      f.put("id", feature.id());
    }
    var props = getSortedNonNullProperties(feature);
    props.put("layer", layerName);
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
    Map<String, Object> map =
        new GsonBuilder().create().fromJson(writer.write(geometry), Map.class);
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
}
