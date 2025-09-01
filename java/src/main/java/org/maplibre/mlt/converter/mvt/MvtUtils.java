package org.maplibre.mlt.converter.mvt;

import org.maplibre.mlt.converter.Settings;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Layer;
import io.github.sebasbaumh.mapbox.vectortile.adapt.jts.MvtReader;
import io.github.sebasbaumh.mapbox.vectortile.adapt.jts.TagKeyValueMapConverter;
import io.github.sebasbaumh.mapbox.vectortile.adapt.jts.model.JtsMvt;
import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.*;
import no.ecc.vectortile.VectorTileDecoder;
import org.locationtech.jts.geom.GeometryFactory;
import org.locationtech.jts.geom.PrecisionModel;
import org.locationtech.jts.geom.impl.PackedCoordinateSequenceFactory;
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

  /* Uses the mapbox-vector-tile-java library for decoding the MVT tile */
  // TODO: combine decodeMvt and decodeMvt2 to get correct features
  // Ids are present but geometries of type polygon can have missing vertices and can be wrong in
  // some cases
  public static MapboxVectorTile decodeMvt2(Path mvtFilePath) throws IOException {
    var mvt = Files.readAllBytes(mvtFilePath);
    return decodeMvt2(mvt);
  }

  /* Uses the java-vector-tile library for decoding the MVT tile */
  public static List<VectorTileDecoder.Feature> decodeMvtFast(byte[] mvtTile) throws IOException {
    VectorTileDecoder mvtDecoder = new VectorTileDecoder();
    mvtDecoder.setAutoScale(false);
    var decodedTile = mvtDecoder.decode(mvtTile);
    return decodedTile.asList();
  }

  /* Uses the mapbox-vector-tile-java library for decoding the MVT tile */
  public static JtsMvt decodeMvt2Fast(ByteArrayInputStream mvtTile) throws IOException {
    final PrecisionModel precisionModel = new PrecisionModel();
    final PackedCoordinateSequenceFactory coordinateSequenceFactory =
        new PackedCoordinateSequenceFactory(PackedCoordinateSequenceFactory.DOUBLE);
    var geometryFactory = new GeometryFactory(precisionModel, 0, coordinateSequenceFactory);
    return MvtReader.loadMvt(mvtTile, geometryFactory, new TagKeyValueMapConverter(true, ID_KEY));
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
    return decodeMvt(mvtTile, Optional.empty());
  }

  public static MapboxVectorTile decodeMvt(
      byte[] mvtTile, Optional<List<ColumnMapping>> columnMappings) throws IOException {
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
        var transformedProperties = transformNestedPropertyNames(properties, columnMappings);

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

  private static MapboxVectorTile decodeMvt2(byte[] mvtTile) throws IOException {
    final PrecisionModel precisionModel = new PrecisionModel();
    final PackedCoordinateSequenceFactory coordinateSequenceFactory =
        new PackedCoordinateSequenceFactory(PackedCoordinateSequenceFactory.DOUBLE);
    var geometryFactory = new GeometryFactory(precisionModel, 0, coordinateSequenceFactory);
    var result =
        MvtReader.loadMvt(
            new ByteArrayInputStream(mvtTile),
            geometryFactory,
            new TagKeyValueMapConverter(true, ID_KEY));
    final var mvtLayers = result.getLayers();

    var layers = new ArrayList<Layer>();
    for (var layer : mvtLayers) {
      var name = layer.getName();
      var mvtFeatures = layer.getGeometries();
      var features = new ArrayList<Feature>();
      for (var mvtFeature : mvtFeatures) {
        Map<?, ?> properties =
            mvtFeature.getUserData() instanceof Map<?, ?> map ? map : new LinkedHashMap<>();
        var id = (long) properties.get(ID_KEY);
        properties.remove(ID_KEY);
        // TODO: quick and dirty -> implement generic
        var transformedProperties = transformNestedPropertyNames(properties, Optional.empty());
        var feature = new Feature(id, mvtFeature, transformedProperties);
        features.add(feature);
      }

      layers.add(new Layer(name, features, layer.getExtent()));
    }

    return new MapboxVectorTile(layers);
  }

  private static Map<String, Object> transformNestedPropertyNames(
      Map<?, ?> properties, Optional<List<ColumnMapping>> columnMappings) {
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

          if (columnMappings.isPresent()) {
            var columnMapping =
                columnMappings.get().stream()
                    .filter(m -> key.startsWith(m.mvtPropertyPrefix()))
                    .findFirst();
            if (columnMapping.isPresent()) {
              var transformedKey =
                  key.replaceAll(
                      columnMapping.get().mvtDelimiterSign(), Settings.MLT_CHILD_FIELD_SEPARATOR);
              transformedProperties.put(transformedKey, v);
              return;
            }
          }

          transformedProperties.put(key, v);
        });
    return transformedProperties;
  }
}
