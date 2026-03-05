package org.maplibre.mlt;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.util.List;
import java.util.Objects;
import org.maplibre.mlt.converter.mvt.MapboxVectorTile;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.MapLibreTile;

public class TestUtils {
  public enum Optimization {
    NONE,
    SORTED,
    IDS_REASSIGNED
  }

  public static void compareTilesSequential(MapLibreTile mlTile, MapboxVectorTile mvTile) {
    var mltLayers = mlTile.layers();
    var mvtLayers = mvTile.layers();

    for (var i = 0; i < mvtLayers.size(); i++) {
      var mltLayer = mltLayers.get(i);
      var mvtLayer = mvtLayers.get(i);
      var mltFeatures = mltLayer.features();
      var mvtFeatures = mvtLayer.features();
      for (org.maplibre.mlt.data.Feature mvtFeature : mvtFeatures) {
        var mltFeature =
            mltFeatures.stream()
                .filter(f -> Objects.equals(f.idOrNull(), mvtFeature.idOrNull()))
                .findFirst()
                .get();
        assertEquals(mvtFeature.id(), mltFeature.id());

        var mltGeometry = mltFeature.geometry();
        var mvtGeometry = mvtFeature.geometry();
        assertEquals(mvtGeometry, mltGeometry);

        var mltProperties = mltFeature.properties();
        var mvtProperties = mvtFeature.properties();
        for (var mvtProperty : mvtProperties.entrySet()) {
          var mltProperty = mltProperties.get(mvtProperty.getKey());
          assertEquals(mvtProperty.getValue(), mltProperty);
        }
      }
    }
  }

  private static int compareFeatures(
      List<Feature> mltFeatures, List<Feature> mvtFeatures, boolean isFeatureTableSorted) {
    int numErrors = 0;
    assertEquals(
        mvtFeatures.size(),
        mltFeatures.size(),
        "mvtFeatures.size()=" + mvtFeatures.size() + " mltFeatures.size()=" + mltFeatures.size());
    for (var j = 0; j < mltFeatures.size(); j++) {
      var mltFeature = mltFeatures.get(j);
      var mvtFeature =
          isFeatureTableSorted
              ? mvtFeatures.stream()
                  .filter(f -> f.hasId() == mltFeature.hasId() && f.id() == mltFeature.id())
                  .findFirst()
                  .get()
              : mvtFeatures.get(j);

      // TODO: add id again
      /*assertEquals(
      mvtFeature.id(),
      mltFeature.id(),
      "mvtFeature.id()=" + mvtFeature.id() + " mltFeature.id()=" + mltFeature.id());*/

      var mltGeometry = mltFeature.geometry();
      var mvtGeometry = mvtFeature.geometry();
      if (!mvtGeometry.equalsExact(mltGeometry)) {
        numErrors++;
      }
      var mltProperties = mltFeature.properties();
      var mvtProperties = mvtFeature.properties();
      for (var mvtProperty : mvtProperties.entrySet()) {
        var mvtPropertyKey = mvtProperty.getKey();
        // TODO: remove the below special case once this bug is fixed:
        // https://github.com/maplibre/maplibre-tile-spec/issues/181
        if (mvtPropertyKey.equals("id")) {
          continue;
        }
        var mltProperty = mltProperties.get(mvtPropertyKey);
        if (mltProperty == null) {
          numErrors++;
        } else if (!mltProperty.equals(mvtProperty.getValue())) {
          numErrors++;
        } else {
          assertEquals(
              mvtProperty.getValue(),
              mltProperty,
              "mvtProperty.getValue()=" + mvtProperty.getValue() + " mltProperty=" + mltProperty);
        }
      }
    }
    return numErrors;
  }

  public static int compareTilesSequential(
      MapLibreTile mlTile, MapboxVectorTile mvTile, boolean isFeatureTableSorted) {
    int numErrors = 0;
    var mltLayers = mlTile.layers();
    var mvtLayers = mvTile.layers();
    assertEquals(
        mltLayers.size(),
        mvtLayers.size(),
        "mltLayers.size()=" + mltLayers.size() + " mvtLayers.size()=" + mvtLayers.size());
    for (var i = 0; i < mvtLayers.size(); i++) {
      var mvtLayer = mvtLayers.get(i);
      var mvtFeatures = mvtLayer.features();
      var mltLayer = mltLayers.get(i);
      var mltFeatures = mltLayer.features();
      numErrors += compareFeatures(mltFeatures, mvtFeatures, isFeatureTableSorted);
    }
    return numErrors;
  }
}
