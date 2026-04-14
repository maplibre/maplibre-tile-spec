package org.maplibre.mlt.json;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.Strictness;
import com.google.gson.reflect.TypeToken;
import java.lang.reflect.Type;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.SortedMap;
import java.util.TreeMap;
import java.util.stream.Collectors;
import java.util.stream.StreamSupport;
import org.locationtech.jts.geom.Geometry;
import org.locationtech.jts.io.geojson.GeoJsonWriter;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Layer;
import org.maplibre.mlt.data.MapLibreTile;
import org.maplibre.mlt.data.MapboxVectorTile;
import org.maplibre.mlt.data.Property;

/** Utility for converting MVT and MLT tiles to JSON and GeoJSON. */
public final class Json {
  // GeoJSON does not support non-numeric floats; use Rust-style string tokens for
  // cross-implementation consistency.
  private static final String F32_NAN = "f32::NAN";
  private static final String F32_INFINITY = "f32::INFINITY";
  private static final String F32_NEG_INFINITY = "f32::NEG_INFINITY";
  private static final String F64_NAN = "f64::NAN";
  private static final String F64_INFINITY = "f64::INFINITY";
  private static final String F64_NEG_INFINITY = "f64::NEG_INFINITY";

  private static final Type linkedHashMapType =
      new TypeToken<LinkedHashMap<String, Object>>() {}.getType();

  private Json() {}

  /**
   * Convert a MapboxVectorTile to a JSON string with the same structure as the MVT spec. The
   * geometry is represented as a WKT string for comparison with MLT output. Note that this is not
   * GeoJSON and is intended for testing/debugging purposes only.
   *
   * @param tile the tile to convert
   * @param pretty if true, the output will be pretty-printed, otherwise, it will be compact
   * @return a JSON string representing the tile
   */
  public static String toJson(MapboxVectorTile tile, boolean pretty) {
    return createGson(pretty).toJson(toJsonObjects(tile));
  }

  /**
   * Convert a MapLibreTile to a JSON string with the same structure as the MLT spec. The geometry
   * is represented as a WKT string for comparison with MVT output. Note that this is not GeoJSON
   * and is intended for testing/debugging purposes only.
   *
   * @param tile the tile to convert
   * @param pretty if true, the output will be pretty-printed, otherwise, it will be compact
   * @return a JSON string representing the tile
   */
  public static String toJson(MapLibreTile tile, boolean pretty) {
    return createGson(pretty).toJson(toJsonObjects(tile));
  }

  /**
   * Convert a MapLibreTile to a GeoJSON string. The layer name and tile extent are included as
   * properties with keys "_layer" and "_extent", respectively, for testing/debugging purposes.
   *
   * @param tile the tile to convert
   * @param pretty if true, the output will be pretty-printed, otherwise, it will be compact
   * @return a GeoJSON string representing the tile
   */
  public static String toGeoJson(MapLibreTile tile, boolean pretty) {
    final var gson = createGson(pretty);
    return gson.toJson(toGeoJsonObjects(tile, gson));
  }

  private static Gson createGson(boolean pretty) {
    final var builder =
        new GsonBuilder()
            .disableJdkUnsafe()
            .disableHtmlEscaping()
            // not .serializeNulls()
            .serializeSpecialFloatingPointValues()
            .setStrictness(Strictness.STRICT);
    return (pretty ? builder.setPrettyPrinting() : builder).create();
  }

  private static Object floatToken(Float value) {
    if (value.isNaN()) {
      return F32_NAN;
    } else if (value == Float.POSITIVE_INFINITY) {
      return F32_INFINITY;
    } else if (value == Float.NEGATIVE_INFINITY) {
      return F32_NEG_INFINITY;
    }
    return value;
  }

  private static Object doubleToken(Double value) {
    if (value.isNaN()) {
      return F64_NAN;
    } else if (value == Double.POSITIVE_INFINITY) {
      return F64_INFINITY;
    } else if (value == Double.NEGATIVE_INFINITY) {
      return F64_NEG_INFINITY;
    }
    return value;
  }

  /** Recursively replace Float/Double NaN and +/-Infinity with GeoJSON string tokens. */
  private static Object floatsAsStrings(Object obj) {
    return switch (obj) {
      case Float value -> floatToken(value);
      case Double value -> doubleToken(value);
      case Iterable<?> iterable ->
          StreamSupport.stream(iterable.spliterator(), false).map(Json::floatsAsStrings).toList();
      case Map<?, ?> map ->
          map.entrySet().stream()
              .collect(
                  Collectors.toMap(
                      Map.Entry::getKey,
                      entry -> floatsAsStrings(entry.getValue()),
                      Json::failOnDuplicate,
                      LinkedHashMap::new));
      default -> obj;
    };
  }

  private static Map<String, Object> toJsonObjects(MapLibreTile mlTile) {
    return Map.of("layers", mlTile.getLayerStream().map(Json::toJson).toList());
  }

  private static Map<String, Object> toJson(Layer layer) {
    final var map = new LinkedHashMap<String, Object>();
    map.put("name", layer.name());
    map.put("extent", layer.tileExtent());
    map.put("features", layer.features().stream().map(Json::toJson).toList());
    return map;
  }

  private static Map<String, Object> toJson(Feature feature) {
    final var featureMap = new LinkedHashMap<String, Object>();
    if (feature.hasId()) {
      featureMap.put("id", feature.getId());
    }
    featureMap.put("geometry", feature.getGeometry().toString());

    // Keep non-null properties only to facilitate direct comparison with MVT output.
    final var propertyMap =
        feature
            .getPropertyStream()
            .filter(entry -> entry.getValue(feature.getIndex()) != null)
            .collect(
                Collectors.toMap(
                    Property::getName,
                    p -> p.getValue(feature.getIndex()),
                    Json::failOnDuplicate,
                    LinkedHashMap::new));

    featureMap.put("properties", propertyMap);
    return featureMap;
  }

  private static Map<String, Object> toGeoJsonObjects(MapLibreTile mlTile, Gson gson) {
    final var featureCollectionMap = new LinkedHashMap<String, Object>();
    featureCollectionMap.put("type", "FeatureCollection");
    featureCollectionMap.put(
        "features",
        mlTile
            .getLayerStream()
            .flatMap(
                layer ->
                    layer.features().stream()
                        .map(feature -> featureToGeoJson(layer, feature, gson)))
            .toList());
    return featureCollectionMap;
  }

  private static Map<String, Object> featureToGeoJson(Layer layer, Feature feature, Gson gson) {
    final var featureMap = new LinkedHashMap<String, Object>();
    featureMap.put("type", "Feature");
    if (feature.hasId()) {
      featureMap.put("id", feature.getId());
    }

    final var props = getSortedNonNullProperties(feature);
    props.put("_layer", layer.name());
    props.put("_extent", layer.tileExtent());
    featureMap.put("properties", floatsAsStrings(props));

    final var geom = feature.getGeometry();
    featureMap.put("geometry", geom == null ? null : geometryToGeoJson(geom, gson));
    return featureMap;
  }

  private static Object failOnDuplicate(Object a, Object b) {
    throw new IllegalStateException("Duplicate key");
  }

  private static SortedMap<String, Object> getSortedNonNullProperties(Feature feature) {
    return feature
        .getPropertyStream()
        .filter(entry -> entry.getValue(feature.getIndex()) != null)
        .collect(
            Collectors.toMap(
                Property::getName,
                p -> p.getValue(feature.getIndex()),
                Json::failOnDuplicate,
                TreeMap::new));
  }

  private static Map<String, Object> geometryToGeoJson(Geometry geometry, Gson gson) {
    var writer = new GeoJsonWriter();
    writer.setEncodeCRS(false);

    final LinkedHashMap<String, Object> map =
        gson.fromJson(writer.write(geometry), linkedHashMapType);

    if (map.containsKey("coordinates")) {
      map.put("coordinates", intifyCoordinates(map.get("coordinates")));
    }
    return map;
  }

  /** Recursively convert whole-number doubles to longs inside a coordinates structure. */
  private static Object intifyCoordinates(Object obj) {
    if (obj instanceof Iterable<?> list) {
      return StreamSupport.stream(list.spliterator(), false).map(Json::intifyCoordinates).toList();
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
    return Map.of("layers", mvTile.getLayerStream().map(Json::toJson).toList());
  }
}
