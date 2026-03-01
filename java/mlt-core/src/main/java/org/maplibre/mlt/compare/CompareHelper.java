package org.maplibre.mlt.compare;

import java.util.HashSet;
import java.util.Map;
import java.util.Objects;
import java.util.Optional;
import java.util.Set;
import java.util.function.Predicate;
import java.util.regex.Pattern;
import java.util.stream.Collectors;
import org.apache.commons.lang3.tuple.Pair;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;
import org.locationtech.jts.geom.Geometry;
import org.maplibre.mlt.converter.mvt.MapboxVectorTile;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Layer;
import org.maplibre.mlt.data.MapLibreTile;

public class CompareHelper {

  public enum CompareMode {
    LayersOnly,
    Geometry,
    Properties,
    All,
  }

  public record Difference(
      String message,
      Optional<Integer> layerIndex,
      Optional<String> layerName,
      Optional<Integer> featureIndex,
      Optional<Pair<String, String>> items,
      Optional<Pair<Layer, Layer>> layers,
      Optional<Pair<Feature, Feature>> features,
      Optional<Pair<Object, Object>> propertyValues,
      Optional<Pair<Geometry, Geometry>> geometries) {
    @Override
    public String toString() {
      final var itemStr =
          items.isPresent()
              ? ("MVT: " + items.get().getLeft() + " MLT: " + items.get().getRight())
              : "";
      final var propStr =
          propertyValues.isPresent()
              ? ("MVT value: "
                  + String.valueOf(propertyValues.get().getLeft())
                  + " MLT value: "
                  + String.valueOf(propertyValues.get().getRight()))
              : "";
      final var geomStr =
          geometries.isPresent()
              ? ("MVT geometry:\n"
                  + geometries.get().getLeft()
                  + "\nMLT geometry:\n"
                  + geometries.get().getRight())
              : "";
      return (message
          + (itemStr.isEmpty() ? "" : " " + itemStr)
          + (propStr.isEmpty() ? "" : " " + propStr)
          + (geomStr.isEmpty() ? "" : "\n" + geomStr)
          + (layerIndex.isPresent() ? (" at layer index " + layerIndex.get()) : "")
          + (layerName.isPresent() ? (" in layer '" + layerName.get() + "': ") : "")
          + (featureIndex.isPresent() ? (" at feature index " + featureIndex.get()) : ""));
    }

    static Builder builder(@NotNull String message) {
      return new Builder().message(message);
    }

    static class Builder {

      private @NotNull String message = "";
      private Optional<Integer> layerIndex = Optional.empty();
      private Optional<String> layerName = Optional.empty();
      private Optional<Integer> featureIndex = Optional.empty();
      private Optional<Pair<Layer, Layer>> layers = Optional.empty();
      private Optional<Pair<Feature, Feature>> features = Optional.empty();
      private Optional<Pair<String, String>> items = Optional.empty();
      private Optional<Pair<Object, Object>> propertyValues = Optional.empty();
      private Optional<Pair<Geometry, Geometry>> geometries = Optional.empty();

      public Builder message(@NotNull String message) {
        this.message = message;
        return this;
      }

      public Builder layerIndex(int layerIndex) {
        this.layerIndex = Optional.of(layerIndex);
        return this;
      }

      public Builder layerName(String layerName) {
        this.layerName = Optional.of(layerName);
        return this;
      }

      public Builder featureIndex(int featureIndex) {
        this.featureIndex = Optional.of(featureIndex);
        return this;
      }

      public Builder items(@NotNull String mvt, @NotNull String mlt) {
        this.items = Optional.of(Pair.of(mvt, mlt));
        return this;
      }

      public Builder layers(@NotNull Layer mvt, @NotNull Layer mlt) {
        this.layers = Optional.of(Pair.of(mvt, mlt));
        return this;
      }

      public Builder features(@NotNull Feature mvt, @NotNull Feature mlt) {
        this.features = Optional.of(Pair.of(mvt, mlt));
        return this;
      }

      public Builder propertyValues(@Nullable Object mvt, @Nullable Object mlt) {
        this.propertyValues = Optional.of(Pair.of(mvt, mlt));
        return this;
      }

      public Builder geometries(@Nullable Geometry mvt, @Nullable Geometry mlt) {
        this.geometries = Optional.of(Pair.of(mvt, mlt));
        return this;
      }

      public Difference build() {
        return new Difference(
            message,
            layerIndex,
            layerName,
            featureIndex,
            items,
            layers,
            features,
            propertyValues,
            geometries);
      }
    }
  }

  /// Compare the content of MLT and MVT tiles.
  /// Returns a single difference, stopping immediately when the first one is found.
  /// @param mlTile The MLT tile
  /// @param mbTile The MVT tile
  /// @param compareMode Which parts of the tiles to compare
  /// @return A description of the first difference found, or an empty Optional if the tiles are
  // equal.
  public static Optional<Difference> compareTiles(
      @NotNull MapLibreTile mlTile,
      @NotNull MapboxVectorTile mvTile,
      @NotNull CompareMode compareMode) {
    return compareTiles(mlTile, mvTile, compareMode, null, false);
  }

  /// Compare the content of MLT and MVT tiles.
  /// Returns a single difference, stopping immediately when the first one is found.
  /// @param mlTile The MLT tile
  /// @param mbTile The MVT tile
  /// @param compareMode Which parts of the tiles to compare
  /// @param layerFilter A regex pattern to filter layers by name. If null, all layers are compared.
  /// @param filterInvert If true, only layers *not*  matching the filter are compared.
  /// @return A description of the first difference found, or an empty Optional if the tiles are
  // equal.
  public static Optional<Difference> compareTiles(
      @NotNull MapLibreTile mlTile,
      @NotNull MapboxVectorTile mvTile,
      @NotNull CompareMode compareMode,
      @Nullable Pattern layerFilter,
      boolean filterInvert) {
    final Predicate<Layer> filter =
        (layerFilter == null)
            ? x -> true
            : x -> layerFilter.matcher(x.name()).matches() ^ filterInvert;
    return compareTiles(mlTile, mvTile, compareMode, filter);
  }

  /// Compare the content of MLT and MVT tiles
  /// Returns a single difference, stopping immediately when the first one is found.
  /// @param mlTile The MLT tile
  /// @param mbTile The MVT tile
  /// @param compareMode Which parts of the tiles to compare
  /// @param layerFilter A filter to select which layers to compare.
  /// @return A description of the first difference found, or an empty Optional if the tiles are
  // equal.
  public static Optional<Difference> compareTiles(
      @NotNull MapLibreTile mlTile,
      @NotNull MapboxVectorTile mvTile,
      @NotNull CompareMode compareMode,
      @NotNull Predicate<Layer> layerFilter) {
    final var mvtLayers =
        mvTile.layers().stream().filter(x -> !x.features().isEmpty()).filter(layerFilter).toList();
    final var mltLayers =
        mlTile.layers().stream().filter(x -> !x.features().isEmpty()).filter(layerFilter).toList();
    if (mltLayers.size() != mvtLayers.size()) {
      final var mvtNames = mvtLayers.stream().map(Layer::name).collect(Collectors.joining(", "));
      final var mltNames = mltLayers.stream().map(Layer::name).collect(Collectors.joining(", "));
      return Optional.of(
          Difference.builder("Number of layers in MLT and MVT tiles do not match")
              .items(mvtNames, mltNames)
              .build());
    }
    for (var i = 0; i < mvtLayers.size(); i++) {
      final var mltLayer = mltLayers.get(i);
      final var mvtLayer = mvtLayers.get(i);
      final var layerResult = compareLayer(mltLayer, mvtLayer, compareMode, i);
      if (layerResult.isPresent()) {
        return layerResult;
      }
    }
    return Optional.empty();
  }

  private static Optional<Difference> compareLayer(
      Layer mltLayer, Layer mvtLayer, @NotNull CompareMode compareMode, int layerIndex) {
    final var mltFeatures = mltLayer.features();
    final var mvtFeatures = mvtLayer.features();
    if (!mltLayer.name().equals(mvtLayer.name())) {
      return Optional.of(
          Difference.builder("Layer names differ")
              .layerIndex(layerIndex)
              .items(mvtLayer.name(), mltLayer.name())
              .build());
    }
    if (mltFeatures.size() != mvtFeatures.size()) {
      return Optional.of(
          Difference.builder("Number of features differ")
              .items(String.valueOf(mvtFeatures.size()), String.valueOf(mltFeatures.size()))
              .layerIndex(layerIndex)
              .layerName(mvtLayer.name())
              .build());
    }
    for (var j = 0; j < mvtFeatures.size(); j++) {
      final var mvtFeature = mvtFeatures.get(j);
      // Expect features to be written in the same order
      final var mltFeature = mltFeatures.get(j);
      final var featureResult =
          compareFeature(mltFeature, mvtFeature, compareMode, j, mvtLayer.name());
      if (featureResult.isPresent()) {
        return featureResult;
      }
    }
    return Optional.empty();
  }

  private static Optional<Difference> compareFeature(
      Feature mltFeature,
      Feature mvtFeature,
      @NotNull CompareMode compareMode,
      int featureIndex,
      String layerName) {
    if (!Objects.equals(mvtFeature.idOrNull(), mltFeature.idOrNull())) {
      return Optional.of(
          Difference.builder("Feature IDs differ")
              .layerName(layerName)
              .featureIndex(featureIndex)
              .items(String.valueOf(mvtFeature.id()), String.valueOf(mltFeature.id()))
              .build());
    }
    if (compareMode == CompareMode.Geometry || compareMode == CompareMode.All) {
      final var geomResult = compareGeometry(mltFeature, mvtFeature, featureIndex, layerName);
      if (geomResult.isPresent()) {
        return geomResult;
      }
    }
    if (compareMode == CompareMode.Properties || compareMode == CompareMode.All) {
      final var propResult = compareProperties(mltFeature, mvtFeature, featureIndex, layerName);
      if (propResult.isPresent()) {
        return propResult;
      }
    }
    return Optional.empty();
  }

  private static Optional<Difference> compareGeometry(
      Feature mltFeature, Feature mvtFeature, int featureIndex, String layerName) {
    final var mltGeometry = mltFeature.geometry();
    final var mltGeomValid = mltGeometry.isValid();
    final var mvtGeometry = mvtFeature.geometry();
    final var mvtGeomValid = mvtGeometry.isValid();
    if (mltGeomValid != mvtGeomValid) {
      return Optional.of(
          Difference.builder("Geometry validity does not match")
              .layerName(layerName)
              .featureIndex(featureIndex)
              .items(mvtGeomValid ? "valid" : "invalid", mltGeomValid ? "valid" : "invalid")
              .build());
    }

    if (mvtGeomValid && !mltGeometry.equals(mvtGeometry)) {
      return Optional.of(
          Difference.builder("Geometries do not match")
              .layerName(layerName)
              .featureIndex(featureIndex)
              .geometries(mvtGeometry, mltGeometry)
              .build());
    }

    return Optional.empty();
  }

  private static boolean propertyValuesEqual(Object a, Object b) {
    // Try simple equality
    if (Objects.equals(a, b)) {
      return true;
    }
    // Allow for, e.g., int32 and int64 representations of the same number by comparing strings
    return a.toString().equals(b.toString());
  }

  private static Optional<Difference> compareProperties(
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
      return Optional.of(
          Difference.builder("Property keys do not match")
              .layerName(layerName)
              .featureIndex(featureIndex)
              .items(mvtKeyStr, mltKeyStr)
              .build());
    }
    // compare values
    final var unequalKeys =
        mvtProperties.keySet().stream()
            .filter(key -> !propertyValuesEqual(mvtProperties.get(key), mltProperties.get(key)))
            .toList();
    if (!unequalKeys.isEmpty()) {
      final var mvtValues =
          unequalKeys.stream()
              .map(key -> key + ": " + mvtProperties.get(key))
              .collect(Collectors.joining(", "));
      final var mltValues =
          unequalKeys.stream()
              .map(key -> key + ": " + mltProperties.get(key))
              .collect(Collectors.joining(", "));
      return Optional.of(
          Difference.builder("Property values do not match")
              .layerName(layerName)
              .featureIndex(featureIndex)
              .items(mvtValues, mltValues)
              .build());
    }
    return Optional.empty();
  }

  /// Returns the values that are in set a but not in set b
  private static <T> Set<T> getAsymmetricSetDiff(Set<T> a, Set<T> b) {
    Set<T> diff = new HashSet<>(a);
    diff.removeAll(b);
    return diff;
  }
}
