package org.maplibre.mlt.json;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import java.util.HashMap;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.locationtech.jts.geom.Coordinate;
import org.locationtech.jts.geom.Geometry;
import org.locationtech.jts.geom.GeometryFactory;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Layer;
import org.maplibre.mlt.data.MLTFeature;
import org.maplibre.mlt.data.MapLibreTile;
import org.maplibre.mlt.data.MapboxVectorTile;

class JsonTest {
  private static final GeometryFactory GEOMETRY_FACTORY = new GeometryFactory();

  @Test
  void propertyValues() {
    final Map<String, Object> properties = new HashMap<>();
    properties.put("name", "Main St");
    properties.put("nullValue", null);

    final var tile =
        mltOf(
            layer(
                "roads",
                4096,
                feature(7, GEOMETRY_FACTORY.createPoint(new Coordinate(1, 2)), properties)));
    final JsonObject root = JsonParser.parseString(Json.toJson(tile, false)).getAsJsonObject();
    final var layer = root.getAsJsonArray("layers").get(0).getAsJsonObject();
    final var feature = layer.getAsJsonArray("features").get(0).getAsJsonObject();

    assertEquals("roads", layer.get("name").getAsString());
    assertEquals(4096, layer.get("extent").getAsInt());
    assertEquals(7L, feature.get("id").getAsLong());
    assertEquals("POINT (1 2)", feature.get("geometry").getAsString());
    assertEquals("Main St", feature.getAsJsonObject("properties").get("name").getAsString());
    assertFalse(feature.getAsJsonObject("properties").has("nullValue"));
  }

  @Test
  void mvtSerializesLayers() {
    final var tile =
        mvtOf(
            layer(
                "water",
                4096,
                feature(
                    GEOMETRY_FACTORY.createPoint(new Coordinate(5, 6)), Map.of("class", "river"))));

    final JsonObject root = JsonParser.parseString(Json.toJson(tile, false)).getAsJsonObject();
    final var layer = root.getAsJsonArray("layers").get(0).getAsJsonObject();

    assertEquals("water", layer.get("name").getAsString());
    assertEquals(1, layer.getAsJsonArray("features").size());
    assertEquals(
        "river",
        layer
            .getAsJsonArray("features")
            .get(0)
            .getAsJsonObject()
            .getAsJsonObject("properties")
            .get("class")
            .getAsString());
  }

  @Test
  void featureCollectionWithLayerMetadata() {
    final var tile =
        mltOf(
            layer(
                "buildings",
                8192,
                feature(
                    10,
                    GEOMETRY_FACTORY.createPoint(new Coordinate(12, 34)),
                    Map.of("height", 15))));

    final JsonObject root = JsonParser.parseString(Json.toGeoJson(tile, false)).getAsJsonObject();
    final var feature = root.getAsJsonArray("features").get(0).getAsJsonObject();
    final var properties = feature.getAsJsonObject("properties");

    assertEquals("FeatureCollection", root.get("type").getAsString());
    assertEquals("Feature", feature.get("type").getAsString());
    assertEquals(10L, feature.get("id").getAsLong());
    assertEquals("buildings", properties.get("_layer").getAsString());
    assertEquals(8192, properties.get("_extent").getAsInt());
    assertEquals(15, properties.get("height").getAsInt());
    assertEquals("Point", feature.getAsJsonObject("geometry").get("type").getAsString());
  }

  @Test
  void floatAndDoublePropertiesToTokens() {
    final var nested = new LinkedHashMap<String, Object>();
    nested.put("inner", Double.NEGATIVE_INFINITY);

    final var properties = new LinkedHashMap<String, Object>();
    properties.put("f32nan", Float.NaN);
    properties.put("f32inf", Float.POSITIVE_INFINITY);
    properties.put("f32neg", Float.NEGATIVE_INFINITY);
    properties.put("f64nan", Double.NaN);
    properties.put("f64inf", Double.POSITIVE_INFINITY);
    properties.put("f64neg", Double.NEGATIVE_INFINITY);
    properties.put("f32", 1.5f);
    properties.put("f64", -1.5);

    final var tile =
        mltOf(
            layer(
                "roads",
                4096,
                feature(1, GEOMETRY_FACTORY.createPoint(new Coordinate(0, 0)), properties)));

    final var props =
        JsonParser.parseString(Json.toGeoJson(tile, false))
            .getAsJsonObject()
            .getAsJsonArray("features")
            .get(0)
            .getAsJsonObject()
            .getAsJsonObject("properties");

    assertEquals("1.5", props.get("f32").getAsJsonPrimitive().toString());
    assertEquals("f32::NAN", props.get("f32nan").getAsString());
    assertEquals("f32::INFINITY", props.get("f32inf").getAsString());
    assertEquals("f32::NEG_INFINITY", props.get("f32neg").getAsString());
    assertEquals("-1.5", props.get("f64").getAsJsonPrimitive().toString());
    assertEquals("f64::NAN", props.get("f64nan").getAsString());
    assertEquals("f64::INFINITY", props.get("f64inf").getAsString());
    assertEquals("f64::NEG_INFINITY", props.get("f64neg").getAsString());
  }

  @Test
  void nullGeometry() {
    final var tile = mltOf(layer("places", 4096, feature(5, null, Map.of("name", "unknown"))));

    final var feature =
        JsonParser.parseString(Json.toGeoJson(tile, false))
            .getAsJsonObject()
            .getAsJsonArray("features")
            .get(0)
            .getAsJsonObject();

    assertFalse(feature.has("geometry"));
  }

  @Test
  void integerCoordinates() {
    final var line =
        GEOMETRY_FACTORY.createLineString(
            new Coordinate[] {new Coordinate(1.0, 2.0), new Coordinate(3.5, 4.0)});
    final var tile = mltOf(layer("roads", 4096, feature(1, line, Map.of())));

    final var coordinates =
        JsonParser.parseString(Json.toGeoJson(tile, false))
            .getAsJsonObject()
            .getAsJsonArray("features")
            .get(0)
            .getAsJsonObject()
            .getAsJsonObject("geometry")
            .getAsJsonArray("coordinates");

    assertEquals("1", coordinates.get(0).getAsJsonArray().get(0).getAsJsonPrimitive().toString());
    assertEquals("2", coordinates.get(0).getAsJsonArray().get(1).getAsJsonPrimitive().toString());
    assertEquals("3.5", coordinates.get(1).getAsJsonArray().get(0).getAsJsonPrimitive().toString());
    assertEquals("4", coordinates.get(1).getAsJsonArray().get(1).getAsJsonPrimitive().toString());
  }

  @Test
  void propertiesAlphabetical() {
    final var properties = new LinkedHashMap<String, Object>();
    properties.put("zeta", 3);
    properties.put("alpha", 1);
    properties.put("middle", 2);

    final var tile =
        mltOf(
            layer(
                "roads",
                4096,
                feature(1, GEOMETRY_FACTORY.createPoint(new Coordinate(0, 0)), properties)));

    final var json = Json.toGeoJson(tile, false);
    final var propertiesStart = json.indexOf("\"properties\":{");
    final var geometryStart = json.indexOf(",\"geometry\"", propertiesStart);
    final var propertiesJson = json.substring(propertiesStart, geometryStart);

    final var alphaIndex = propertiesJson.indexOf("\"alpha\":");
    final var middleIndex = propertiesJson.indexOf("\"middle\":");
    final var zetaIndex = propertiesJson.indexOf("\"zeta\":");

    assertTrue(alphaIndex >= 0);
    assertTrue(middleIndex >= 0);
    assertTrue(zetaIndex >= 0);
    assertTrue(alphaIndex < middleIndex);
    assertTrue(middleIndex < zetaIndex);
  }

  @Test
  void jsonElementStableOrder() {
    final var tile =
        mltOf(
            layer(
                "roads",
                4096,
                feature(
                    7, GEOMETRY_FACTORY.createPoint(new Coordinate(1, 2)), Map.of("name", "A"))));

    final var json = Json.toJson(tile, false);

    final var layersIndex = json.indexOf("\"layers\":");
    final var nameIndex = json.indexOf("\"name\":", layersIndex);
    final var extentIndex = json.indexOf("\"extent\":", nameIndex);
    final var featuresIndex = json.indexOf("\"features\":", extentIndex);
    final var idIndex = json.indexOf("\"id\":", featuresIndex);
    final var geometryIndex = json.indexOf("\"geometry\":", idIndex);
    final var propertiesIndex = json.indexOf("\"properties\":", geometryIndex);

    assertTrue(layersIndex >= 0);
    assertTrue(nameIndex > layersIndex);
    assertTrue(extentIndex > nameIndex);
    assertTrue(featuresIndex > extentIndex);
    assertTrue(idIndex > featuresIndex);
    assertTrue(geometryIndex > idIndex);
    assertTrue(propertiesIndex > geometryIndex);
  }

  private static MapLibreTile mltOf(Layer... layers) {
    return new MapLibreTile(List.of(layers));
  }

  private static MapboxVectorTile mvtOf(Layer... layers) {
    return new MapboxVectorTile(List.of(layers));
  }

  private static Layer layer(String name, int extent, Feature... features) {
    return new Layer(name, List.of(features), extent);
  }

  private static Feature feature(Geometry geometry, Map<String, Object> properties) {
    return MLTFeature.builder().geometry(geometry).rawProperties(properties).index(0).build();
  }

  private static Feature feature(long id, Geometry geometry, Map<String, Object> properties) {
    return MLTFeature.builder()
        .id(id)
        .geometry(geometry)
        .rawProperties(properties)
        .index(0)
        .build();
  }
}
