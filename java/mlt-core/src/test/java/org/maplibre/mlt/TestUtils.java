package org.maplibre.mlt;

import java.util.Map;
import java.util.SequencedCollection;
import java.util.stream.Collectors;
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
            .map(layer -> filterLayers(layer, filter))
            .toList(),
        tile.tileId());
  }

  private static @NotNull Layer filterLayers(@NotNull Layer layer, @NotNull TileFilter filter) {
    return new Layer(
        layer.name(),
        layer.features().stream()
            .filter(feature -> filter.test(layer, feature))
            .flatMap(StreamUtil.ofType(MVTFeature.class))
            .map(feature -> filterFeatures(layer, feature, filter))
            .toList(),
        layer.tileExtent());
  }

  private static @NotNull Feature filterFeatures(
      @NotNull Layer layer, @NotNull MVTFeature feature, @NotNull TileFilter filter) {
    return feature.toBuilder()
        .properties(
            feature.getRawProperties().entrySet().stream()
                .filter(p -> filter.test(layer, feature, p.getKey(), p.getValue()))
                .collect(Collectors.toMap(Map.Entry::getKey, Map.Entry::getValue)))
        .build();
  }
}
