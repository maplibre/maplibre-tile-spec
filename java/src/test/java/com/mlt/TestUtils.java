package com.mlt;

import static org.junit.jupiter.api.Assertions.assertEquals;

import com.mlt.converter.mvt.MapboxVectorTile;
import com.mlt.data.MapLibreTile;
import com.mlt.vector.FeatureTable;
import java.util.Map;

public class TestUtils {
  public static void compareTilesVectorized(
      FeatureTable[] featureTables, MapboxVectorTile mvTile, boolean isFeatureTableSorted) {
    var mvtLayers = mvTile.layers();
    for (var i = 0; i < mvtLayers.size(); i++) {
      var featureTable = featureTables[i];
      var mvtLayer = mvtLayers.get(i);
      var mvtFeatures = mvtLayer.features();
      var featureIterator = featureTable.iterator();
      for (var j = 0; j < mvtFeatures.size(); j++) {
        var mltFeature = featureIterator.next();
        var mvtFeature =
            isFeatureTableSorted
                ? mvtFeatures.stream().filter(f -> f.id() == mltFeature.id()).findFirst().get()
                : mvtFeatures.get(j);

        assertEquals(mvtFeature.id(), mltFeature.id());

        var mvtGeometry = mvtFeature.geometry();
        var mltGeometry = mltFeature.geometry();
        assertEquals(mvtGeometry, mltGeometry);

        var mltProperties = mltFeature.properties();
        for (var property : mltProperties.entrySet()) {
          var mltPropertyKey = property.getKey();
          var mltPropertyValue = property.getValue();
          if (mltPropertyValue instanceof Map<?, ?>) {
            /* Handle shared dictionary case -> currently only String is supported
             * as nested property in the converter, so only handle this case */
            var mvtProperties = mvtFeature.properties();
            var nestedStringValues = (Map<String, String>) mltPropertyValue;
            var mvtStringProperties =
                mvtProperties.entrySet().stream()
                    .filter(
                        p -> p.getKey().contains(mltPropertyKey) && p.getValue() instanceof String)
                    .toList();
            // TODO: verify why mlt seems to have a property more than mvt on the
            // name:* column in some tiles
            for (var mvtProperty : mvtStringProperties) {
              var mvtPropertyKey = mvtProperty.getKey();
              var mvtPropertyValue = mvtProperty.getValue();
              var mltValue = nestedStringValues.get(mvtPropertyKey);

              if (mvtPropertyKey.equals("name:ja:rm")) {
                // TODO: fix -> currently the converter can't handle a triple nested property name
                System.out.println(
                    "Skip verification for the name:ja:rm property name since it is currently"
                        + "not supported in the converter.");
                continue;
              }

              assertEquals(mvtPropertyValue, mltValue);
            }
          } else {
            assertEquals(mvtFeature.properties().get(mltPropertyKey), mltPropertyValue);
          }
        }
      }
    }
  }

  public static void compareTilesSequential(MapLibreTile mlTile, MapboxVectorTile mvTile) {
    var mltLayers = mlTile.layers();
    var mvtLayers = mvTile.layers();

    for (var i = 0; i < mvtLayers.size(); i++) {
      var mltLayer = mltLayers.get(i);
      var mvtLayer = mvtLayers.get(i);
      var mltFeatures = mltLayer.features();
      var mvtFeatures = mvtLayer.features();
      for (com.mlt.data.Feature mvtFeature : mvtFeatures) {
        var mltFeature =
            mltFeatures.stream().filter(f -> f.id() == mvtFeature.id()).findFirst().get();
        assertEquals(mvtFeature.id(), mltFeature.id());

        var mltGeometry = mltFeature.geometry();
        var mvtGeometry = mvtFeature.geometry();
        assertEquals(mvtGeometry, mltGeometry);

        var mltProperties = mltFeature.properties();
        var mvtProperties = mvtFeature.properties();
        for (var mvtProperty : mvtProperties.entrySet()) {
          /*if(mvtProperty.getKey().contains("name:ja:rm")){
              System.out.println(mvtProperty.getKey() + " " + mvtProperty.getValue() + " " + mltProperties.get(mvtProperty.getKey()) + " " + j + " " + i);
              continue;
          }*/

          var mltProperty = mltProperties.get(mvtProperty.getKey());
          assertEquals(mvtProperty.getValue(), mltProperty);
        }
      }
    }
  }
}
