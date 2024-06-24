package com.mlt;

import static org.junit.jupiter.api.Assertions.assertEquals;

import com.mlt.converter.mvt.MapboxVectorTile;
import com.mlt.data.Feature;
import com.mlt.data.MapLibreTile;
import com.mlt.vector.FeatureTable;
import java.util.ArrayList;
import java.util.List;

public class TestUtils {

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
              ? mvtFeatures.stream().filter(f -> f.id() == mltFeature.id()).findFirst().get()
              : mvtFeatures.get(j);
      assertEquals(
          mvtFeature.id(),
          mltFeature.id(),
          "mvtFeature.id()=" + mvtFeature.id() + " mltFeature.id()=" + mltFeature.id());
      var mltGeometry = mltFeature.geometry();
      var mvtGeometry = mvtFeature.geometry();
      if (!mvtGeometry.equalsExact(mltGeometry)) {
        // System.out.println("Failure comparing geometry for feature: " + mvtFeature.id());
        // System.out.println("    mvtGeometry: " + mvtGeometry);
        // System.out.println("    mltGeometry: " + mltGeometry);
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
          // System.out.println(mvtFeature.id() + " mlt prop missing for " + mvtProperty.getKey());
          // System.out.println("  mvtProperties: " + mvtProperties);
          // System.out.println("  mltProperties: " + mltProperties);
          numErrors++;
        } else if (!mltProperty.equals(mvtProperty.getValue())) {
          // System.out.println(
          //     "Failure comparing property "
          //         + mvtProperty.getKey()
          //         + " for feature: "
          //         + mvtFeature.id());
          // System.out.println("    mvtProperty: " + mvtProperty.getValue());
          // System.out.println("    mltProperty: " + mltProperty);
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

  public static int compareTilesVectorized(
      FeatureTable[] featureTables, MapboxVectorTile mvTile, boolean isFeatureTableSorted) {
    int numErrors = 0;
    var mvtLayers = mvTile.layers();
    assertEquals(
        featureTables.length,
        mvtLayers.size(),
        "featureTables.length=" + featureTables.length + " mvtLayers.size()=" + mvtLayers.size());
    for (var i = 0; i < mvtLayers.size(); i++) {
      var mvtLayer = mvtLayers.get(i);
      var mvtFeatures = mvtLayers.get(i).features();
      var featureTable = featureTables[i];
      var featureIterator = featureTable.iterator();
      var mltFeatures = new ArrayList<Feature>();
      while (featureIterator.hasNext()) {
        mltFeatures.add(featureIterator.next());
      }
      numErrors += compareFeatures(mltFeatures, mvtFeatures, isFeatureTableSorted);
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
      var mvtFeatures = mvtLayers.get(i).features();
      var mltLayer = mltLayers.get(i);
      var mltFeatures = mltLayer.features();
      numErrors += compareFeatures(mltFeatures, mvtFeatures, isFeatureTableSorted);
    }
    return numErrors;
  }
}
