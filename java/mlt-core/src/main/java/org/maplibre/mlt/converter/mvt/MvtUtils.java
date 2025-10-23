package org.maplibre.mlt.converter.mvt;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.*;
import java.util.regex.Pattern;
import no.ecc.vectortile.VectorTileDecoder;
import org.maplibre.mlt.converter.Settings;
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
  static Map<String, VectorTileLayer> decodeMvtMapbox(byte[] mvtTile) throws IOException {
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
        // TODO: quick and dirty -> implement generic
        final var mappings =
            columnMappings.entrySet().stream()
                .filter(entry -> entry.getKey().matcher(layerName).matches())
                .flatMap(entry -> entry.getValue().stream())
                .toList();
        var transformedProperties = transformNestedPropertyNames(properties, mappings);

        var feature =
            new Feature(mvtFeature.getId(), mvtFeature.getGeometry(), transformedProperties);
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

  static Map<String, Object> transformNestedPropertyNames(
      Map<?, ?> properties, List<ColumnMapping> columnMappings) {
    var transformedProperties = new LinkedHashMap<String, Object>();
    properties.forEach(
        (k, v) -> {
          /* Currently id is not supported as a property name in a FeatureTable,
           *  so this quick workaround is implemented */
          // TODO: refactor -> implement proper solution
          final var key = k.toString();
          if (k.equals(ID_KEY)) {
            transformedProperties.put("_" + ID_KEY, v);
            return;
          }

          if (!columnMappings.isEmpty()) {
            final var columnMapping =
                columnMappings.stream().filter(m -> key.startsWith(m.getPrefix())).findFirst();
            if (columnMapping.isPresent()) {
              final var transformedKey =
                  key.replaceAll(
                      columnMapping.get().getDelimiter(), Settings.MLT_CHILD_FIELD_SEPARATOR);
              transformedProperties.put(transformedKey, v);
              return;
            }
          }

          transformedProperties.put(key, v);
        });
    return transformedProperties;
  }
}
