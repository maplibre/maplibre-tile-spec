package org.maplibre.mlt.compare;

import java.util.HashSet;
import java.util.Map;
import java.util.Objects;
import java.util.Set;
import java.util.function.Predicate;
import java.util.regex.Pattern;
import java.util.stream.Collectors;
import org.apache.commons.lang3.tuple.Pair;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;
import org.maplibre.mlt.converter.mvt.MapboxVectorTile;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Layer;
import org.maplibre.mlt.data.MapLibreTile;

public class CompareHelper {
  ///  Compare the content of MLT and MVT tiles
  /// @param mlTile The MLT tile
  /// @param mbTile The MVT tile
  /// @param compareGeom Whether to compare geometries.
  /// @param compareProp Whether to compare properties.
  /// @param layerFilter A regex pattern to filter layers by name. If null, all layers are compared.
  /// @param filterInvert If true, only layers *not*  matching the filter are compared.
  /// @return a pair of (areEqual, errorMessage). If the tiles are equal, errorMessage is null
  public static Pair<Boolean, String> compareTiles(
      @NotNull MapLibreTile mlTile,
      @NotNull MapboxVectorTile mvTile,
      boolean compareGeom,
      boolean compareProp,
      @Nullable Pattern layerFilter,
      boolean filterInvert) {
    final Predicate<Layer> filter =
        (layerFilter == null)
            ? x -> true
            : x -> layerFilter.matcher(x.name()).matches() ^ filterInvert;
    return compareTiles(mlTile, mvTile, compareGeom, compareProp, filter);
  }

  /// Compare the content of MLT and MVT tiles
  /// @param mlTile The MLT tile
  /// @param mbTile The MVT tile
  /// @param compareGeom Whether to compare geometries.
  /// @param compareProp Whether to compare properties.
  /// @param layerFilter A filter to select which layers to compare.
  /// @return a pair of (areEqual, errorMessage). If the tiles are equal, errorMessage is null
  public static Pair<Boolean, String> compareTiles(
      @NotNull MapLibreTile mlTile,
      @NotNull MapboxVectorTile mvTile,
      boolean compareGeom,
      boolean compareProp,
      @NotNull Predicate<Layer> layerFilter) {
    final var mvtLayers =
        mvTile.layers().stream().filter(x -> !x.features().isEmpty()).filter(layerFilter).toList();
    final var mltLayers = mlTile.layers();
    if (mltLayers.size() != mvtLayers.size()) {
      final var mvtNames = mvtLayers.stream().map(Layer::name).collect(Collectors.joining(", "));
      final var mltNames = mltLayers.stream().map(Layer::name).collect(Collectors.joining(", "));
      return Pair.of(
          false,
          "Number of layers in MLT and MVT tiles do not match:\nMVT:\n"
              + mvtNames
              + "\nMLT:\n"
              + mltNames);
    }
    for (var i = 0; i < mvtLayers.size(); i++) {
      final var mltLayer = mltLayers.get(i);
      final var mvtLayer = mvtLayers.get(i);
      final var layerResult =
          compareLayer(mltLayer, mvtLayer, compareGeom, compareProp, i, mltLayer.name());
      if (!layerResult.getLeft()) {
        return layerResult;
      }
    }
    return Pair.of(true, null);
  }

  private static Pair<Boolean, String> compareLayer(
      Layer mltLayer,
      Layer mvtLayer,
      boolean compareGeom,
      boolean compareProp,
      int featureIndex,
      String layerName) {
    final var mltFeatures = mltLayer.features();
    final var mvtFeatures = mvtLayer.features();
    if (!mltLayer.name().equals(mvtLayer.name())) {
      return Pair.of(
          false,
          "Layer index "
              + featureIndex
              + " of MVT and MLT tile differ: '"
              + mvtLayer.name()
              + "' != '"
              + mltLayer.name()
              + "'");
    }
    if (mltFeatures.size() != mvtFeatures.size()) {
      return Pair.of(
          false,
          "Number of features in MLT and MVT layer '"
              + mvtLayer.name()
              + "' do not match: "
              + mltFeatures.size()
              + " != "
              + mvtFeatures.size());
    }
    for (var j = 0; j < mvtFeatures.size(); j++) {
      final var mvtFeature = mvtFeatures.get(j);
      // Expect features to be written in the same order
      final var mltFeature = mltFeatures.get(j);
      final var featureResult =
          compareFeature(mltFeature, mvtFeature, compareGeom, compareProp, j, mvtLayer.name());
      if (!featureResult.getLeft()) {
        return featureResult;
      }
    }
    return Pair.of(true, null);
  }

  private static Pair<Boolean, String> compareFeature(
      Feature mltFeature,
      Feature mvtFeature,
      boolean compareGeom,
      boolean compareProp,
      int featureIndex,
      String layerName) {
    if (mvtFeature.id() != mltFeature.id()) {
      return Pair.of(
          false,
          "Feature IDs for index "
              + featureIndex
              + " in layer '"
              + layerName
              + "' do not match: "
              + mvtFeature.id()
              + " != "
              + mltFeature.id());
    }
    if (compareGeom) {
      final var geomResult = compareGeometry(mltFeature, mvtFeature, featureIndex, layerName);
      if (!geomResult.getLeft()) {
        return geomResult;
      }
    }
    if (compareProp) {
      final var propResult = compareProperties(mltFeature, mvtFeature, featureIndex, layerName);
      if (!propResult.getLeft()) {
        return propResult;
      }
    }
    return Pair.of(true, null);
  }

  private static Pair<Boolean, String> compareGeometry(
      Feature mltFeature, Feature mvtFeature, int featureIndex, String layerName) {
    final var mltGeometry = mltFeature.geometry();
    final var mltGeomValid = mltGeometry.isValid();
    final var mvtGeometry = mvtFeature.geometry();
    final var mvtGeomValid = mvtGeometry.isValid();
    if (mltGeomValid != mvtGeomValid) {
      return Pair.of(
          false,
          "Geometry validity in MLT and MVT layers do not match for feature index "
              + featureIndex
              + " in layer '"
              + layerName
              + "': \nMVT:\n"
              + mvtGeomValid
              + " : "
              + mvtGeometry
              + "\nMLT:\n"
              + mltGeomValid
              + " : "
              + mltGeometry);
    }

    if (mvtGeomValid && !mltGeometry.equals(mvtGeometry)) {
      return Pair.of(
          false,
          "Geometries in MLT and MVT layers do not match for feature index "
              + featureIndex
              + " in layer '"
              + layerName
              + "': \nMVT:\n"
              + mvtGeometry
              + "\nMLT:\n"
              + mltGeometry
              + "\nDifference:\n"
              + mvtGeometry.difference(mltGeometry));
    }

    return Pair.of(true, null);
  }

  private static boolean propertyValuesEqual(Object a, Object b) {
    // Try simple equality
    if (Objects.equals(a, b)) {
      return true;
    }
    if (a instanceof Double && b instanceof Float) {
      // We currently encode doubles as floats
      return ((Double) a).floatValue() == (Float) b;
    }
    // Allow for, e.g., int32 and int64 representations of the same number by comparing strings
    return a.toString().equals(b.toString());
  }

  private static Pair<Boolean, String> compareProperties(
      Feature mltFeature, Feature mvtFeature, int featureIndex, String layerName) {
    final var mltProperties = mltFeature.properties();
    final var mvtProperties = mvtFeature.properties();
    final var mvtPropertyKeys = mvtProperties.keySet();
    final var nonNullMLTKeys =
        mltProperties.entrySet().stream()
            .filter(entry -> entry.getValue() != null)
            .map(Map.Entry::getKey)
            .collect(Collectors.toUnmodifiableSet());
    // compare keys
    if (!mvtPropertyKeys.equals(nonNullMLTKeys)) {
      final var mvtKeys = getAsymmetricSetDiff(mvtPropertyKeys, nonNullMLTKeys);
      final var mvtKeyStr = mvtKeys.isEmpty() ? "(none)" : String.join(", ", mvtKeys);
      final var mltKeys = getAsymmetricSetDiff(nonNullMLTKeys, mvtPropertyKeys);
      final var mltKeyStr = mltKeys.isEmpty() ? "(none)" : String.join(", ", mltKeys);
      return Pair.of(
          false,
          "Property keys in MLT and MVT feature index "
              + featureIndex
              + " in layer '"
              + layerName
              + "' do not match:\nOnly in MVT: "
              + mvtKeyStr
              + "\nOnly in MLT: "
              + mltKeyStr);
    }
    // compare values
    final var unequalKeys =
        mvtProperties.keySet().stream()
            .filter(key -> !propertyValuesEqual(mvtProperties.get(key), mltProperties.get(key)))
            .toList();
    if (!unequalKeys.isEmpty()) {
      final var unequalValues =
          unequalKeys.stream()
              .map(
                  key ->
                      "  "
                          + key
                          + ": mvt="
                          + mvtProperties.get(key)
                          + ", mlt="
                          + mltProperties.get(key)
                          + "\n")
              .collect(Collectors.joining());
      return Pair.of(
          false,
          "Property values in MLT and MVT feature index "
              + featureIndex
              + " in layer '"
              + layerName
              + "' do not match: \n"
              + unequalValues);
    }
    return Pair.of(true, null);
  }

  /// Returns the values that are in set a but not in set b
  private static <T> Set<T> getAsymmetricSetDiff(Set<T> a, Set<T> b) {
    Set<T> diff = new HashSet<>(a);
    diff.removeAll(b);
    return diff;
  }
}
