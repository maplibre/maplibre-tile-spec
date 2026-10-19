package org.maplibre.mlt.converter.mvt;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.BitSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
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
        var ignored = feature.loadGeometry();
      }
    }

    return vectorTile.layers;
  }

  public static MapboxVectorTile decodeMvt(byte[] mvtTile) throws IOException {
    VectorTileDecoder mvtDecoder = new VectorTileDecoder();
    mvtDecoder.setAutoScale(false);

    final var featureHasId = getFeatureIdPresence(mvtTile);
    final var featuresByLayer = new LinkedHashMap<String, List<DecodedFeature>>();
    var featureIndex = 0;
    for (var mvtFeature : mvtDecoder.decode(mvtTile)) {
      featuresByLayer
          .computeIfAbsent(mvtFeature.getLayerName(), k -> new ArrayList<>())
          .add(new DecodedFeature(mvtFeature, featureHasId.get(featureIndex++)));
    }

    var layers = new ArrayList<Layer>(featuresByLayer.size());
    for (var layerGroup : featuresByLayer.entrySet()) {
      final var layerName = layerGroup.getKey();
      final var layerFeatures = layerGroup.getValue();

      var tileExtent = 0;
      final var features = new ArrayList<Feature>(layerFeatures.size());
      for (var decodedFeature : layerFeatures) {
        final var mvtFeature = decodedFeature.feature();
        features.add(
            MVTFeature.builder()
                .index(features.size())
                .id(decodedFeature.hasId() ? mvtFeature.getId() : null)
                .geometry(mvtFeature.getGeometry())
                .properties(mvtFeature.getAttributes())
                .build());
        tileExtent = Math.max(tileExtent, mvtFeature.getExtent());
      }

      layers.add(new Layer(layerName, features, tileExtent));
    }

    return new MapboxVectorTile(layers);
  }

  private record DecodedFeature(VectorTileDecoder.Feature feature, boolean hasId) {}

  /// java-vector-tile reports a missing feature ID as 0, so read which features have one, in the
  /// order in which its decoder returns them
  private static BitSet getFeatureIdPresence(byte[] mvtTile) throws IOException {
    final var hasId = new BitSet();
    var index = 0;
    for (var layer : vector_tile.VectorTile.Tile.parseFrom(mvtTile).getLayersList()) {
      for (var feature : layer.getFeaturesList()) {
        hasId.set(index++, feature.hasId());
      }
    }
    return hasId;
  }
}
