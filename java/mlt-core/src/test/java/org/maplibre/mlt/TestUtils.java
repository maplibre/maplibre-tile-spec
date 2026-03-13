package org.maplibre.mlt;

import java.util.Map;
import java.util.Optional;
import java.util.SequencedCollection;
import java.util.function.Function;
import java.util.stream.Collectors;
import java.util.stream.Stream;
import org.jetbrains.annotations.NotNull;
import org.maplibre.mlt.compare.CompareHelper;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Layer;
import org.maplibre.mlt.data.MVTFeature;
import org.maplibre.mlt.data.MapLibreTile;
import org.maplibre.mlt.data.MapboxVectorTile;
import org.maplibre.mlt.util.StreamUtil;

public class TestUtils {
  public enum Optimization {
    NONE,
    SORTED,
    IDS_REASSIGNED
  }

  private static int compareFeatures(
      SequencedCollection<Feature> mltFeatures,
      SequencedCollection<Feature> mvtFeatures,
      boolean allowFeatureSort) {
    final var difference =
        CompareHelper.compareFeatures(
            mltFeatures, mvtFeatures, CompareHelper.CompareMode.All, 0, "test", allowFeatureSort);
    return difference.isPresent() ? 1 : 0;
  }

  public static int compareTilesSequential(
      MapLibreTile mlTile, MapboxVectorTile mvTile, boolean allowFeatureSort) {
    return StreamUtil.zip(
            mlTile.getLayerStream(),
            mvTile.getLayerStream(),
            (mltLayer, mvtLayer) ->
                compareFeatures(mltLayer.features(), mvtLayer.features(), allowFeatureSort))
        .reduce(0, Integer::sum);
  }

  /// Helper method to filter and cast stream elements, for use with `Stream.flatMap`
  public static <Target extends Base, Base> Function<Base, Stream<Target>> ofType(
      @SuppressWarnings("SameParameterValue") Class<Target> targetType) {
    return value ->
        targetType.isInstance(value) ? Stream.of(targetType.cast(value)) : Stream.empty();
  }

  /// Helper method to filter Optional values by type, for use with `Optional.flatMap`
  public static <Target extends Base, Base> Function<Base, Optional<Target>> optionalOfType(
      @SuppressWarnings("SameParameterValue") Class<Target> targetType) {
    return value ->
        targetType.isInstance(value) ? Optional.of(targetType.cast(value)) : Optional.empty();
  }

  public static interface TileFilter {
    default boolean test(Layer layer, Feature feature, String propertyKey, Object propertyValue) {
      return true;
    }

    default boolean test(Layer layer) {
      return true;
    }

    default boolean test(Layer layer, Feature feature) {
      return true;
    }
  }

  // Filter a tile by layer, feature, and/or property.
  public static MapboxVectorTile filterTile(
      @NotNull MapboxVectorTile tile, @NotNull TileFilter filter) {
    return new MapboxVectorTile(
        tile.getLayerStream()
            .filter(layer -> filter.test(layer))
            .map(
                layer ->
                    new Layer(
                        layer.name(),
                        layer.features().stream()
                            .filter(feature -> filter.test(layer, feature))
                            .flatMap(ofType(MVTFeature.class))
                            .map(
                                feature ->
                                    feature
                                        .asBuilder()
                                        .properties(
                                            feature.getRawProperties().entrySet().stream()
                                                .filter(
                                                    p ->
                                                        filter.test(
                                                            layer,
                                                            feature,
                                                            p.getKey(),
                                                            p.getValue()))
                                                .collect(
                                                    Collectors.toMap(
                                                        Map.Entry::getKey, Map.Entry::getValue)))
                                        .build())
                            .toList(),
                        layer.tileExtent()))
            .toList(),
        tile.tileId());
  }
}
