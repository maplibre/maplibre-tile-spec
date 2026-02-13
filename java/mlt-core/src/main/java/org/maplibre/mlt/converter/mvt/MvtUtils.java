package org.maplibre.mlt.converter.mvt;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.*;
import java.util.regex.Pattern;
import no.ecc.vectortile.VectorTileDecoder;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Layer;
import org.springmeyer.Pbf;
import org.springmeyer.VectorTile;
import org.springmeyer.VectorTileLayer;

public class MvtUtils {
  private static final String ID_KEY = "id";

  /* Uses the java-vector-tile library for decoding the MVT tile */
  public static MapboxVectorTile decodeMvt(Path mvtFilePath) throws IOException {
    var mvt = Files.readAllBytes(mvtFilePath);
    return decodeMvt(mvt);
  }

  /* Uses the java-vector-tile library for decoding the MVT tile */
  public static List<VectorTileDecoder.Feature> decodeMvtFast(byte[] mvtTile) throws IOException {
    VectorTileDecoder mvtDecoder = new VectorTileDecoder();
    mvtDecoder.setAutoScale(false);
    var decodedTile = mvtDecoder.decode(mvtTile);
    return decodedTile.asList();
  }

  /* Use the java port of the vector-tile-js library created by Dane Springmeyer.
   * To get realistic number and have a fair comparison in terms of the decoding performance,
   * the geometries of the features have to be decoded.
   * */
  public static Map<String, VectorTileLayer> decodeMvtMapbox(byte[] mvtTile) throws IOException {
    Pbf pbf = new Pbf(mvtTile);
    VectorTile vectorTile = new VectorTile(pbf, pbf.length);
    for (var layer : vectorTile.layers.values()) {
      for (int i = 0; i < layer.length; i++) {
        var feature = layer.feature(i);
        var geometry = feature.loadGeometry();
      }
    }

    return vectorTile.layers;
  }

  public static MapboxVectorTile decodeMvt(byte[] mvtTile) throws IOException {
    return decodeMvt(mvtTile, Map.of());
  }

  public static MapboxVectorTile decodeMvt(
      byte[] mvtTile, Map<Pattern, List<ColumnMapping>> columnMappings) throws IOException {
    VectorTileDecoder mvtDecoder = new VectorTileDecoder();
    mvtDecoder.setAutoScale(false);

    var tile = mvtDecoder.decode(mvtTile);
    var mvtFeatures = tile.asList();
    var layers = new ArrayList<Layer>();
    for (var layerName : tile.getLayerNames()) {
      var layerFeatures =
          mvtFeatures.stream().filter(f -> f.getLayerName().equals(layerName)).toList();

      var features = new ArrayList<Feature>();
      var tileExtent = 0;
      for (var mvtFeature : layerFeatures) {
        var properties = new HashMap<>(mvtFeature.getAttributes());
        var feature = new Feature(mvtFeature.getId(), mvtFeature.getGeometry(), properties);
        features.add(feature);

        var featureTileExtent = mvtFeature.getExtent();
        if (featureTileExtent > tileExtent) {
          tileExtent = featureTileExtent;
        }
      }

      layers.add(new Layer(layerName, features, tileExtent));
    }

    return new MapboxVectorTile(layers);
  }
}
