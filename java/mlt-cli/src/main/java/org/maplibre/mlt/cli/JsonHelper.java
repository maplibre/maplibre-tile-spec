package org.maplibre.mlt.cli;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import java.util.stream.Collectors;
import org.locationtech.jts.geom.Geometry;
import org.locationtech.jts.io.geojson.GeoJsonWriter;
import org.maplibre.mlt.converter.mvt.MapboxVectorTile;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Layer;
import org.maplibre.mlt.data.MapLibreTile;

public class JsonHelper {
  private static Gson createGson(boolean pretty) {
    var builder = new GsonBuilder().serializeSpecialFloatingPointValues();
    if (pretty) {
      builder.setPrettyPrinting();
    }
    return builder.create();
  }

  public static String toJson(MapLibreTile mlTile) {
    return toJson(mlTile, true);
  }

  public static String toJson(MapLibreTile mlTile, boolean pretty) {
    return createGson(pretty).toJson(toJsonObjects(mlTile));
  }

  public static Map<String, Object> toJsonObjects(MapLibreTile mlTile) {
    return Map.of("layers", mlTile.layers().stream().map(JsonHelper::toJson).toList());
  }

  private static Map<String, Object> toJson(Layer layer) {
    final var map = new TreeMap<String, Object>();
    map.put("name", layer.name());
    map.put("extent", layer.tileExtent());
    map.put("features", layer.features().stream().map(JsonHelper::toJson).toList());
    return map;
  }

  private static Map<String, Object> toJson(Feature feature) {
    final var map = new TreeMap<String, Object>();
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

  public static String toGeoJson(MapLibreTile mlTile) {
    return toGeoJson(mlTile, true);
  }

  public static String toGeoJson(MapLibreTile mlTile, boolean pretty) {
    final var gson = createGson(pretty);
    return gson.toJson(toGeoJsonObjects(mlTile, gson));
  }

  private static Map<String, Object> toGeoJsonObjects(MapLibreTile mlTile, Gson gson) {
    final var fc = new TreeMap<String, Object>();
    fc.put("type", "FeatureCollection");
    fc.put(
        "features",
        mlTile.layers().stream()
            .flatMap(
                layer ->
                    layer.features().stream()
                        .map(feature -> featureToGeoJson(layer, feature, gson)))
            .toList());
    return fc;
  }

  private static Map<String, Object> featureToGeoJson(Layer layer, Feature feature, Gson gson) {
    var f = new TreeMap<String, Object>();
    f.put("type", "Feature");
    if (feature.hasId()) {
      f.put("id", feature.id());
    }
    var props = getSortedNonNullProperties(feature);
    props.put("_layer", layer.name());
    props.put("_extent", layer.tileExtent());
    f.put("properties", props);
    var geom = feature.geometry();
    f.put("geometry", geom == null ? null : geometryToGeoJson(geom, gson));
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
  private static Map<String, Object> geometryToGeoJson(Geometry geometry, Gson gson) {
    final var writer = new GeoJsonWriter();
    writer.setEncodeCRS(false);
    final var map = gson.fromJson(writer.write(geometry), Map.class);
    if (map.containsKey("coordinates")) {
      map.put("coordinates", intifyCoordinates(map.get("coordinates")));
    }
    return map;
  }

  /** Recursively convert whole-number doubles to longs inside a coordinates structure. */
  private static Object intifyCoordinates(Object obj) {
    if (obj instanceof List<?> list) {
      return list.stream().map(JsonHelper::intifyCoordinates).toList();
    }
    if (obj instanceof Double d && d == Math.floor(d) && !Double.isInfinite(d)) {
      return d.longValue();
    }
    return obj;
  }

  public static String toJson(MapboxVectorTile mvTile) {
    return toJson(mvTile, true);
  }

  public static String toJson(MapboxVectorTile mvTile, boolean pretty) {
    return createGson(pretty).toJson(toJsonObjects(mvTile));
  }

  private static Map<Object, Object> toJsonObjects(MapboxVectorTile mvTile) {
    return Map.of("layers", mvTile.layers().stream().map(JsonHelper::toJson).toList());
  }
}
