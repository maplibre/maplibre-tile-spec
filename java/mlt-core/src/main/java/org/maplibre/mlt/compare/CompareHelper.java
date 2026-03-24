package org.maplibre.mlt.compare;

import jakarta.annotation.Nullable;
import java.util.Comparator;
import java.util.HashSet;
import java.util.Objects;
import java.util.Optional;
import java.util.SequencedCollection;
import java.util.Set;
import java.util.function.Predicate;
import java.util.regex.Pattern;
import java.util.stream.Collectors;
import lombok.Builder;
import org.apache.commons.lang3.tuple.Pair;
import org.jetbrains.annotations.NotNull;
import org.locationtech.jts.geom.Geometry;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Layer;
import org.maplibre.mlt.data.MapLibreTile;
import org.maplibre.mlt.data.MapboxVectorTile;
import org.maplibre.mlt.data.Property;
import org.maplibre.mlt.util.StreamUtil;

public final class CompareHelper {
  private CompareHelper() {}

  public enum CompareMode {
    None,
    Layers,
    Geometry,
    Properties,
    All
  }

  @Builder
  public record Difference(
      @NotNull String message,
      @Nullable Integer layerIndex,
      @Nullable String layerName,
      @Nullable Integer featureIndex,
      @Nullable Pair<@NotNull String, @NotNull String> items,
      @Nullable Pair<@NotNull Geometry, @NotNull Geometry> geometries) {
    @Override
    public String toString() {
      final var itemStr =
          (items != null) ? ("MVT: " + items.getLeft() + " MLT: " + items.getRight()) : "";
      final var geomStr =
          (geometries != null)
              ? ("MVT geometry:\n"
                  + geometries.getLeft()
                  + "\nMLT geometry:\n"
                  + geometries.getRight())
              : "";
      return (message
          + (itemStr.isEmpty() ? "" : " " + itemStr)
          + (geomStr.isEmpty() ? "" : "\n" + geomStr)
          + ((layerIndex != null) ? (" at layer index " + layerIndex) : "")
          + ((layerName != null) ? (" in layer '" + layerName + "': ") : "")
          + ((featureIndex != null) ? (" at feature index " + featureIndex) : ""));
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
        mvTile.getLayerStream().filter(x -> !x.features().isEmpty()).filter(layerFilter).toList();
    final var mltLayers =
        mlTile.getLayerStream().filter(x -> !x.features().isEmpty()).filter(layerFilter).toList();
    if (mltLayers.size() != mvtLayers.size()) {
      final var mvtNames = mvtLayers.stream().map(Layer::name).collect(Collectors.joining(", "));
      final var mltNames = mltLayers.stream().map(Layer::name).collect(Collectors.joining(", "));
      return Optional.of(
          Difference.builder()
              .message("Number of layers in MLT and MVT tiles do not match")
              .items(Pair.of(mvtNames, mltNames))
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
          Difference.builder()
              .message("Layer names differ")
              .layerIndex(layerIndex)
              .items(Pair.of(mvtLayer.name(), mltLayer.name()))
              .build());
    }
    return compareFeatures(
        mltFeatures, mvtFeatures, compareMode, layerIndex, mvtLayer.name(), true);
  }

  public static Optional<Difference> compareFeatures(
      SequencedCollection<Feature> mltFeatures,
      SequencedCollection<Feature> mvtFeatures,
      @NotNull CompareMode compareMode,
      int layerIndex,
      String layerName,
      boolean allowFeatureSort) {
    if (mltFeatures.size() != mvtFeatures.size()) {
      return Optional.of(
          Difference.builder()
              .message("Number of features differ")
              .items(
                  Pair.of(String.valueOf(mvtFeatures.size()), String.valueOf(mltFeatures.size())))
              .layerIndex(layerIndex)
              .layerName(layerName)
              .build());
    }

    // Allow features to be sorted by ID and still match if all features have IDs
    final var sortableIDs =
        allowFeatureSort
            && mvtFeatures.stream().allMatch(Feature::hasId)
            && mltFeatures.stream().allMatch(Feature::hasId);
    final var maybeSortedMvtFeatures =
        sortableIDs
            ? mvtFeatures.stream().sorted(Comparator.comparing(Feature::getId))
            : mvtFeatures.stream();
    final var maybeSortedMltFeatures =
        sortableIDs
            ? mltFeatures.stream().sorted(Comparator.comparing(Feature::getId))
            : mltFeatures.stream();

    final var featureIndex = new int[] {0};
    return StreamUtil.zip(
            maybeSortedMltFeatures,
            maybeSortedMvtFeatures,
            (mltFeature, mvtFeature) ->
                compareFeature(mltFeature, mvtFeature, compareMode, featureIndex[0]++, layerName))
        .filter(Optional::isPresent)
        .map(Optional::get)
        .findFirst();
  }

  private static Optional<Difference> compareFeature(
      Feature mltFeature,
      Feature mvtFeature,
      @NotNull CompareMode compareMode,
      int featureIndex,
      String layerName) {
    if (!Objects.equals(mvtFeature.idOrNull(), mltFeature.idOrNull())) {
      return Optional.of(
          Difference.builder()
              .message("Feature IDs differ")
              .layerName(layerName)
              .featureIndex(featureIndex)
              .items(
                  Pair.of(String.valueOf(mvtFeature.getId()), String.valueOf(mltFeature.getId())))
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
    final var mltGeometry = mltFeature.getGeometry();
    final var mltGeomValid = mltGeometry.isValid();
    final var mvtGeometry = mvtFeature.getGeometry();
    final var mvtGeomValid = mvtGeometry.isValid();
    if (mltGeomValid != mvtGeomValid) {
      return Optional.of(
          Difference.builder()
              .message("Geometry validity does not match")
              .layerName(layerName)
              .featureIndex(featureIndex)
              .items(
                  Pair.of(mvtGeomValid ? "valid" : "invalid", mltGeomValid ? "valid" : "invalid"))
              .build());
    }

    if (mvtGeomValid && !mltGeometry.equals(mvtGeometry)) {
      return Optional.of(
          Difference.builder()
              .message("Geometries do not match")
              .layerName(layerName)
              .featureIndex(featureIndex)
              .geometries(Pair.of(mvtGeometry, mltGeometry))
              .build());
    }

    return Optional.empty();
  }

  private static boolean propertyValuesEqual(Property pa, Property pb) {
    if (pa == null || pb == null) {
      return pa == null && pb == null;
    }

    final var a = pa.getValue();
    final var b = pb.getValue();

    // Try simple equality
    if (Objects.equals(a, b)) {
      return true;
    }
    if (a == null || b == null) {
      return false;
    }
    // Allow for, e.g., int32 and int64 representations of the same number by comparing strings
    return a.toString().equals(b.toString());
  }

  private static Optional<Difference> compareProperties(
      Feature mltFeature, Feature mvtFeature, int featureIndex, String layerName) {
    final var nonNullMVTKeys =
        mvtFeature
            .getPropertyStream()
            .filter(p -> p.getValue() != null)
            .map(Property::getName)
            .collect(Collectors.toSet());

    final var nonNullMLTKeys =
        mltFeature
            .getPropertyStream()
            .filter(p -> p.getValue() != null)
            .map(Property::getName)
            .collect(Collectors.toSet());
    // compare keys
    if (!nonNullMVTKeys.equals(nonNullMLTKeys)) {
      final var mvtKeys = getAsymmetricSetDiff(nonNullMVTKeys, nonNullMLTKeys);
      final var mvtKeyStr = mvtKeys.isEmpty() ? "(none)" : String.join(", ", mvtKeys);
      final var mltKeys = getAsymmetricSetDiff(nonNullMLTKeys, nonNullMVTKeys);
      final var mltKeyStr = mltKeys.isEmpty() ? "(none)" : String.join(", ", mltKeys);
      return Optional.of(
          Difference.builder()
              .message("Property keys do not match")
              .layerName(layerName)
              .featureIndex(featureIndex)
              .items(Pair.of(mvtKeyStr, mltKeyStr))
              .build());
    }
    // compare values
    final var unequalKeys =
        mvtFeature
            .getPropertyStream()
            .filter(p -> !propertyValuesEqual(p, mltFeature.findProperty(p.getName()).orElse(null)))
            .map(Property::getName)
            .toList();
    if (!unequalKeys.isEmpty()) {
      final var mvtValues =
          unequalKeys.stream()
              .map(
                  key ->
                      key
                          + ": "
                          + mvtFeature
                              .findProperty(key)
                              .map(Property::getValue)
                              .map(String::valueOf)
                              .orElse("null"))
              .collect(Collectors.joining(", "));
      final var mltValues =
          unequalKeys.stream()
              .map(
                  key ->
                      key
                          + ": "
                          + mltFeature
                              .findProperty(key)
                              .map(Property::getValue)
                              .map(String::valueOf)
                              .orElse("null"))
              .collect(Collectors.joining(", "));
      return Optional.of(
          Difference.builder()
              .message("Property values do not match")
              .layerName(layerName)
              .featureIndex(featureIndex)
              .items(Pair.of(mvtValues, mltValues))
              .build());
    }
    return Optional.empty();
  }

  /// Returns the values that are in set `a` but not in set `b`
  private static <T> Set<T> getAsymmetricSetDiff(Set<T> a, Set<T> b) {
    Set<T> diff = new HashSet<>(a);
    diff.removeAll(b);
    return diff;
  }
}
