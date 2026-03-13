package org.maplibre.mlt.converter.mvt;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.stream.Collectors;
import java.util.stream.StreamSupport;
import no.ecc.vectortile.VectorTileDecoder;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Layer;
import org.maplibre.mlt.data.MVTFeature;
import org.maplibre.mlt.data.MapboxVectorTile;
import org.springmeyer.Pbf;
import org.springmeyer.VectorTile;
import org.springmeyer.VectorTileLayer;

public class MvtUtils {
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
    VectorTileDecoder mvtDecoder = new VectorTileDecoder();
    mvtDecoder.setAutoScale(false);

    final var mvtFeatures = mvtDecoder.decode(mvtTile);
    final var featuresByLayer =
        StreamSupport.stream(mvtFeatures.spliterator(), false)
            .collect(Collectors.groupingBy(f -> f.getLayerName()));

    var layers = new ArrayList<Layer>(featuresByLayer.size());
    for (var layerGroup : featuresByLayer.entrySet()) {
      final var layerName = layerGroup.getKey();
      final var layerFeatures = layerGroup.getValue();

      var tileExtent = 0;
      final var features = new ArrayList<Feature>(layerFeatures.size());
      for (var mvtFeature : layerFeatures) {
        final var feature =
            MVTFeature.builder()
                .id(mvtFeature.getId())
                .geometry(mvtFeature.getGeometry())
                .properties(mvtFeature.getAttributes())
                .build();
        features.add(feature);
        tileExtent = Math.max(tileExtent, mvtFeature.getExtent());
      }

      layers.add(new Layer(layerName, features, tileExtent));
    }

    return new MapboxVectorTile(layers);
  }
}
