package org.maplibre.mlt.compare;

import static org.junit.jupiter.api.Assertions.*;

import java.util.List;
import java.util.Map;
import java.util.regex.Pattern;
import org.locationtech.jts.geom.Geometry;
import org.jetbrains.annotations.NotNull;
import org.junit.jupiter.api.Test;
import org.locationtech.jts.geom.Coordinate;
import org.locationtech.jts.geom.GeometryFactory;
import org.maplibre.mlt.compare.CompareHelper.CompareMode;
import org.maplibre.mlt.converter.mvt.MapboxVectorTile;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Layer;
import org.maplibre.mlt.data.MapLibreTile;

class CompareHelperTest {
  @Test
  void identicalEmptyTilesAreEqual() {
    final var result = CompareHelper.compareTiles(mltOf(), mvtOf(), CompareMode.All);
    assertFalse(result.isPresent());
  }

  @Test
  void identicalTilesWithFeaturesAreEqual() {
    final var layer = createLayer("roads", createFeature(1, Map.of("name", "Main St")));
    final var result = CompareHelper.compareTiles(mltOf(layer), mvtOf(layer), CompareMode.All);
    assertFalse(result.isPresent());
  }

  @Test
  void identicalTilesWithMultipleLayersAreEqual() {
    final var l1 = createLayer("roads", createFeature(1, Map.of("name", "Main St")));
    final var l2 = createLayer("water", createFeature(2, Map.of("class", "river")));
    final var result = CompareHelper.compareTiles(mltOf(l1, l2), mvtOf(l1, l2), CompareMode.All);
    assertFalse(result.isPresent());
  }

  @Test
  void differentLayerCountIsDetected() {
    final var mlt = mltOf(createLayer("roads", createFeature(1, Map.of())));
    final var mvt = mvtOf(createLayer("roads", createFeature(1, Map.of())), createLayer("water", createFeature(2, Map.of())));
    final var result = CompareHelper.compareTiles(mlt, mvt, CompareMode.All);
    assertTrue(result.isPresent());
    assertTrue(result.get().toString().contains("Number of layers"));
  }

  @Test
  void emptyMvtLayersAreIgnoredInLayerCount() {
    final var emptyLayer = createLayer("empty");
    final var roadsLayer = createLayer("roads", createFeature(1, Map.of()));
    final var mvt = mvtOf(emptyLayer, roadsLayer);
    final var mlt = mltOf(roadsLayer);
    final var result = CompareHelper.compareTiles(mlt, mvt, CompareMode.All);
    assertFalse(result.isPresent());
  }

  @Test
  void differentLayerNames() {
    final var mlt = mltOf(createLayer("roads", createFeature(1, Map.of())));
    final var mvt = mvtOf(createLayer("water", createFeature(1, Map.of())));
    final var result = CompareHelper.compareTiles(mlt, mvt, CompareMode.All);
    assertTrue(result.isPresent());
    assertTrue(result.get().toString().contains("Layer names differ"));
  }

  @Test
  void differentFeatureCountInLayer() {
    final var mlt = mltOf(createLayer("roads", createFeature(1, Map.of()), createFeature(2, Map.of())));
    final var mvt = mvtOf(createLayer("roads", createFeature(1, Map.of())));
    final var result = CompareHelper.compareTiles(mlt, mvt, CompareMode.All);
    assertTrue(result.isPresent());
    assertTrue(result.get().toString().contains("Number of features differ"));
  }

  @Test
  void differentFeatureIds() {
    final var mlt = mltOf(createLayer("roads", createFeature(99, Map.of())));
    final var mvt = mvtOf(createLayer("roads", createFeature(1, Map.of())));
    final var result = CompareHelper.compareTiles(mlt, mvt, CompareMode.All);
    assertTrue(result.isPresent());
    assertTrue(result.get().toString().contains("Feature IDs differ"));
  }

  @Test
  void featureWithIdVsFeatureWithoutIdDiffer() {
    final var withId = createLayer("roads", createFeature(1, Map.of()));
    final var withoutId = createLayer("roads", createFeature(Map.of()));
    final var result = CompareHelper.compareTiles(mltOf(withoutId), mvtOf(withId), CompareMode.All);
    assertTrue(result.isPresent());
  }

  @Test
  void featuresWithoutIdAreEqual() {
    final var withId = createLayer("roads", createFeature(Map.of()));
    final var withoutId = createLayer("roads", createFeature(Map.of()));
    final var result = CompareHelper.compareTiles(mltOf(withoutId), mvtOf(withId), CompareMode.All);
    assertFalse(result.isPresent());
  }

  @Test
  void differentGeometriesInGeometryMode() {
    final var point1 = FACTORY.createPoint(new Coordinate(1, 2));
    final var point2 = FACTORY.createPoint(new Coordinate(3, 4));
    final var mlt = mltOf(createLayer("roads", createFeature(1, point1)));
    final var mvt = mvtOf(createLayer("roads", createFeature(1, point2)));
    final var result = CompareHelper.compareTiles(mlt, mvt, CompareMode.Geometry);
    assertTrue(result.isPresent());
    assertTrue(result.get().toString().contains("Geometries do not match"));
  }

  @Test
  void geometryDifferencesAreNotCheckedInLayersOnlyMode() {
    final var point1 = FACTORY.createPoint(new Coordinate(1, 2));
    final var point2 = FACTORY.createPoint(new Coordinate(3, 4));
    final var mlt = mltOf(createLayer("roads", createFeature(1, point1)));
    final var mvt = mvtOf(createLayer("roads", createFeature(1, point2)));
    final var result = CompareHelper.compareTiles(mlt, mvt, CompareMode.LayersOnly);
    assertFalse(result.isPresent());
  }

  @Test
  void geometryDifferencesAreNotCheckedInPropertiesMode() {
    final var point1 = FACTORY.createPoint(new Coordinate(1, 2));
    final var point2 = FACTORY.createPoint(new Coordinate(3, 4));
    final var mlt = mltOf(createLayer("roads", createFeature(1, point1)));
    final var mvt = mvtOf(createLayer("roads", createFeature(1, point2)));
    final var result = CompareHelper.compareTiles(mlt, mvt, CompareMode.Properties);
    assertFalse(result.isPresent());
  }

  @Test
  void differentPropertyKeys() {
    final var mlt = mltOf(createLayer("roads", createFeature(1, Map.of("name", "Main St"))));
    final var mvt = mvtOf(createLayer("roads", createFeature(1, Map.of("ref", "M1"))));
    final var result = CompareHelper.compareTiles(mlt, mvt, CompareMode.Properties);
    assertTrue(result.isPresent());
    assertTrue(result.get().toString().contains("Property keys do not match"));
  }

  @Test
  void differentPropertyValues() {
    final var mlt = mltOf(createLayer("roads", createFeature(1, Map.of("name", "Side St"))));
    final var mvt = mvtOf(createLayer("roads", createFeature(1, Map.of("name", "Main St"))));
    final var result = CompareHelper.compareTiles(mlt, mvt, CompareMode.Properties);
    assertTrue(result.isPresent());
    assertTrue(result.get().toString().contains("Property values do not match"));
  }

  @Test
  void propertyDifferencesNotCheckedInGeometryMode() {
    final var mlt = mltOf(createLayer("roads", createFeature(1, Map.of("name", "Side St"))));
    final var mvt = mvtOf(createLayer("roads", createFeature(1, Map.of("name", "Main St"))));
    final var result = CompareHelper.compareTiles(mlt, mvt, CompareMode.Geometry);
    assertFalse(result.isPresent());
  }

  @Test
  void nullPropertyValueInMltTreatedAsAbsent() {
    final Map<String, Object> mltProps = new java.util.HashMap<>();
    mltProps.put("name", null);
    final var mlt = mltOf(createLayer("roads", new Feature(1, FACTORY.createPoint(new Coordinate(1, 2)), mltProps)));
    final var mvt = mvtOf(createLayer("roads", createFeature(1, Map.of())));
    final var result = CompareHelper.compareTiles(mlt, mvt, CompareMode.Properties);
    assertFalse(result.isPresent());
  }

  @Test
  void doubleInMvtMatchesEquivalentFloatInMlt() {
    final var mltFeature = new Feature(1, FACTORY.createPoint(new Coordinate(1, 2)), Map.of("speed", 1.5f));
    final var mvtFeature = new Feature(1, FACTORY.createPoint(new Coordinate(1, 2)), Map.of("speed", 1.5));
    final var mlt = mltOf(createLayer("roads", mltFeature));
    final var mvt = mvtOf(createLayer("roads", mvtFeature));
    final var result = CompareHelper.compareTiles(mlt, mvt, CompareMode.Properties);
    assertFalse(result.isPresent());
  }

  @Test
  void numericValuesWithSameStringRepresentationEqual() {
    final var mltFeature = new Feature(1, FACTORY.createPoint(new Coordinate(1, 2)), Map.of("pop", 42L));
    final var mvtFeature = new Feature(1, FACTORY.createPoint(new Coordinate(1, 2)), Map.of("pop", 42));
    final var mlt = mltOf(createLayer("roads", mltFeature));
    final var mvt = mvtOf(createLayer("roads", mvtFeature));
    final var result = CompareHelper.compareTiles(mlt, mvt, CompareMode.Properties);
    assertFalse(result.isPresent());
  }

  @Test
  void layerFilterIncludesOnlyMatchingLayers() {
    final var roadsMlt = createLayer("roads", createFeature(1, Map.of("name", "Main St")));
    final var roadsMvt = createLayer("roads", createFeature(1, Map.of("name", "Main St")));
    final var waterMvt = createLayer("water", createFeature(2, Map.of("name", "wrong")));
    final var waterMlt = createLayer("water", createFeature(2, Map.of("name", "wrong")));
    final var result =
        CompareHelper.compareTiles(
            mltOf(roadsMlt, waterMlt), mvtOf(roadsMvt, waterMvt), CompareMode.All,
            Pattern.compile("roads"), false);
    assertFalse(result.isPresent());
  }

  @Test
  void layerFilterExcludesMatchingLayersWhenInverted() {
    final var roadsMlt = createLayer("roads", createFeature(1, Map.of("name", "Main St")));
    final var roadsMvt = createLayer("roads", createFeature(1, Map.of("name", "Main St")));
    final var waterMvt = createLayer("water", createFeature(2, Map.of("name", "wrong")));
    final var waterMlt = createLayer("water", createFeature(2, Map.of("name", "wrong")));
    final var result =
        CompareHelper.compareTiles(
            mltOf(roadsMlt, waterMlt), mvtOf(roadsMvt, waterMvt), CompareMode.All,
            Pattern.compile("water"), true);
    assertFalse(result.isPresent());
  }

  @Test
  void layerFilterDetectsDifferenceInMatchingLayer() {
    final var roadsMlt = createLayer("roads", createFeature(1, Map.of("name", "Side St")));
    final var roadsMvt = createLayer("roads", createFeature(1, Map.of("name", "Main St")));
    final var mlt = mltOf(roadsMlt);
    final var mvt = mvtOf(roadsMvt);
    final var result = CompareHelper.compareTiles(mlt, mvt, CompareMode.All, Pattern.compile("roads"), false);
    assertTrue(result.isPresent());
  }

  @Test
  void nullLayerFilterComparesAllLayers() {
    final var l1 = createLayer("roads", createFeature(1, Map.of("name", "Main St")));
    final var l2 = createLayer("water", createFeature(2, Map.of()));
    final var result = CompareHelper.compareTiles(mltOf(l1, l2), mvtOf(l1, l2), CompareMode.All, null, false);
    assertFalse(result.isPresent());
  }

  @Test
  void differenceMessageIncludesLayerName() {
    final var mlt = mltOf(createLayer("roads", createFeature(1, Map.of("x", "a"))));
    final var mvt = mvtOf(createLayer("roads", createFeature(1, Map.of("x", "b"))));
    final var diff = CompareHelper.compareTiles(mlt, mvt, CompareMode.Properties);
    assertTrue(diff.isPresent());
    assertTrue(diff.get().toString().contains("roads"));
  }

  @Test
  void differenceMessageIncludesFeatureIndex() {
    final var mlt = mltOf(createLayer("roads", createFeature(1, Map.of("x", "ok")), createFeature(2, Map.of("x", "bad"))));
    final var mvt = mvtOf(createLayer("roads", createFeature(1, Map.of("x", "ok")), createFeature(2, Map.of("x", "good"))));
    final var diff = CompareHelper.compareTiles(mlt, mvt, CompareMode.Properties);
    assertTrue(diff.isPresent());
    assertTrue(diff.get().toString().contains("1"));
  }

  private static MapLibreTile mltOf(@NotNull Layer... layers) {
    return new MapLibreTile(List.of(layers));
  }

  private static MapboxVectorTile mvtOf(@NotNull Layer... layers) {
    return new MapboxVectorTile(List.of(layers));
  }

  private static Layer createLayer(@NotNull String name, @NotNull Feature... features) {
    return new Layer(name, List.of(features), 4096);
  }

  private static Feature createFeature(long id, Map<String, Object> props) {
    return new Feature(id, FACTORY.createPoint(new Coordinate(1, 2)), props);
  }

  private static Feature createFeature(long id, Geometry geom) {
    return new Feature(id, geom, Map.of());
  }

  private static Feature createFeature(Map<String, Object> props) {
    return new Feature(FACTORY.createPoint(new Coordinate(1, 2)), props);
  }

  private static final GeometryFactory FACTORY = new GeometryFactory();
}
