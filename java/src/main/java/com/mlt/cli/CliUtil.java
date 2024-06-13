package com.mlt.tools;

import com.mlt.data.MapLibreTile;
import com.mlt.vector.FeatureTable;

public class CliUtil {

  private CliUtil() {}

  public static void decodeFeatureTables(FeatureTable[] featureTables) {
    for (var i = 0; i < featureTables.length; i++) {
      var featureTable = featureTables[i];
      var featureIterator = featureTable.iterator();
      while (featureIterator.hasNext()) {
        var mltFeature = featureIterator.next();
        // Trigger decoding of the feature
        mltFeature.id();
        mltFeature.geometry();
        mltFeature.properties();
      }
    }
  }

  public static void printMLT(MapLibreTile mlTile) {
    var mltLayers = mlTile.layers();
    for (var i = 0; i < mltLayers.size(); i++) {
      var mltLayer = mltLayers.get(i);
      System.out.println(mltLayer.name());
      var mltFeatures = mltLayer.features();
      for (var j = 0; j < mltFeatures.size(); j++) {
        var mltFeature = mltFeatures.get(j);
        System.out.println("  " + mltFeature);
      }
    }
  }

  public static void printMLTVectorized(FeatureTable[] featureTables) {
    for (var i = 0; i < featureTables.length; i++) {
      var featureTable = featureTables[i];
      System.out.println(featureTable.getName());
      var featureIterator = featureTable.iterator();
      while (featureIterator.hasNext()) {
        var mltFeature = featureIterator.next();
        System.out.println("  " + mltFeature);
      }
    }
  }
}
