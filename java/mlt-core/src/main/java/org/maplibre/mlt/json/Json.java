package org.maplibre.mlt.json;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.reflect.TypeToken;
import java.lang.reflect.Type;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.SortedMap;
import java.util.TreeMap;
import org.locationtech.jts.geom.Geometry;
import org.locationtech.jts.io.geojson.GeoJsonWriter;
import org.maplibre.mlt.converter.mvt.MapboxVectorTile;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Layer;
import org.maplibre.mlt.data.MapLibreTile;

public final class Json {
  // GeoJSON does not support non-numeric floats; use Rust-style string tokens for
  // cross-implementation consistency.
  private static final String F32_NAN = "f32::NAN";
  private static final String F32_INFINITY = "f32::INFINITY";
  private static final String F32_NEG_INFINITY = "f32::NEG_INFINITY";
  private static final String F64_NAN = "f64::NAN";
  private static final String F64_INFINITY = "f64::INFINITY";
  private static final String F64_NEG_INFINITY = "f64::NEG_INFINITY";

  private Json() {}

  public static String toJson(MapboxVectorTile tile) {
    return toJson(tile, true);
  }

  public static String toJson(MapboxVectorTile tile, boolean pretty) {
    return createGson(pretty).toJson(toJsonObjects(tile));
  }

  public static String toJson(MapLibreTile tile) {
    return toJson(tile, true);
  }

  public static String toJson(MapLibreTile tile, boolean pretty) {
    return createGson(pretty).toJson(toJsonObjects(tile));
  }

  public static String toGeoJson(MapLibreTile tile) {
    return toGeoJson(tile, true);
  }

  public static String toGeoJson(MapLibreTile tile, boolean pretty) {
    var gson = createGson(pretty);
    return gson.toJson(toGeoJsonObjects(tile, gson));
  }

  private static Gson createGson(boolean pretty) {
    var builder = new GsonBuilder().serializeSpecialFloatingPointValues();
    if (pretty) {
      builder.setPrettyPrinting();
    }
    return builder.create();
  }

  /** Recursively replace Float/Double NaN and +/-Infinity with GeoJSON string tokens. */
  private static Object floatsAsStrings(Object obj) {
    if (obj instanceof Float value) {
      if (value.isNaN()) {
        return F32_NAN;
      }
      if (value == Float.POSITIVE_INFINITY) {
        return F32_INFINITY;
      }
      if (value == Float.NEGATIVE_INFINITY) {
        return F32_NEG_INFINITY;
      }
      return value;
    }

    if (obj instanceof Double value) {
      if (value.isNaN()) {
        return F64_NAN;
      }
      if (value == Double.POSITIVE_INFINITY) {
        return F64_INFINITY;
      }
      if (value == Double.NEGATIVE_INFINITY) {
        return F64_NEG_INFINITY;
      }
      return value;
    }

    if (obj instanceof Map<?, ?> map) {
      var output = new LinkedHashMap<Object, Object>(map.size());
      for (var entry : map.entrySet()) {
        output.put(entry.getKey(), floatsAsStrings(entry.getValue()));
      }
      return output;
    }

    if (obj instanceof List<?> list) {
      return list.stream().map(Json::floatsAsStrings).toList();
    }

    if (obj instanceof Iterable<?> iterable) {
      var output = new java.util.ArrayList<>();
      for (var value : iterable) {
        output.add(floatsAsStrings(value));
      }
      return output;
    }

    return obj;
  }

  private static Map<String, Object> toJsonObjects(MapLibreTile mlTile) {
    return Map.of("layers", mlTile.layers().stream().map(Json::toJson).toList());
  }

  private static Map<String, Object> toJson(Layer layer) {
    var map = new LinkedHashMap<String, Object>();
    map.put("name", layer.name());
    map.put("extent", layer.tileExtent());
    map.put("features", layer.features().stream().map(Json::toJson).toList());
    return map;
  }

  private static Map<String, Object> toJson(Feature feature) {
    var map = new LinkedHashMap<String, Object>();
    if (feature.hasId()) {
      map.put("id", feature.id());
    }
    map.put("geometry", feature.geometry().toString());

    // Keep non-null properties only to facilitate direct comparison with MVT output.
    var properties = new LinkedHashMap<String, Object>();
    for (var entry : feature.properties().entrySet()) {
      if (entry.getValue() != null) {
        properties.put(entry.getKey(), entry.getValue());
      }
    }
    map.put("properties", properties);
    return map;
  }

  private static Map<String, Object> toGeoJsonObjects(MapLibreTile mlTile, Gson gson) {
    var fc = new LinkedHashMap<String, Object>();
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
    var f = new LinkedHashMap<String, Object>();
    f.put("type", "Feature");
    if (feature.hasId()) {
      f.put("id", feature.id());
    }

    var props = getSortedNonNullProperties(feature);
    props.put("_layer", layer.name());
    props.put("_extent", layer.tileExtent());
    f.put("properties", floatsAsStrings(props));

    var geom = feature.geometry();
    f.put("geometry", geom == null ? null : geometryToGeoJson(geom, gson));
    return f;
  }

  private static SortedMap<String, Object> getSortedNonNullProperties(Feature feature) {
    var props = new TreeMap<String, Object>();
    for (var entry : feature.properties().entrySet()) {
      if (entry.getValue() != null) {
        props.putIfAbsent(entry.getKey(), entry.getValue());
      }
    }
    return props;
  }

  private static Map<String, Object> geometryToGeoJson(Geometry geometry, Gson gson) {
    var writer = new GeoJsonWriter();
    writer.setEncodeCRS(false);

    Type mapType = new TypeToken<Map<String, Object>>() {}.getType();
    final Map<String, Object> map = gson.fromJson(writer.write(geometry), mapType);

    if (map.containsKey("coordinates")) {
      map.put("coordinates", intifyCoordinates(map.get("coordinates")));
    }
    return new LinkedHashMap<>(map);
  }

  /** Recursively convert whole-number doubles to longs inside a coordinates structure. */
  private static Object intifyCoordinates(Object obj) {
    if (obj instanceof List<?> list) {
      return list.stream().map(Json::intifyCoordinates).toList();
    }

    if (obj instanceof Double value
        && value == Math.floor(value)
        && !value.isInfinite()
        && !value.isNaN()) {
      return value.longValue();
    }

    return obj;
  }

  private static Map<String, Object> toJsonObjects(MapboxVectorTile mvTile) {
    return Map.of("layers", mvTile.layers().stream().map(Json::toJson).toList());
  }
}
