package com.mlt.converter.mvt;

import io.github.sebasbaumh.mapbox.vectortile.adapt.jts.MvtReader;
import io.github.sebasbaumh.mapbox.vectortile.adapt.jts.TagKeyValueMapConverter;
import io.github.sebasbaumh.mapbox.vectortile.adapt.jts.model.JtsLayer;
import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.*;
import java.util.stream.Collectors;
import no.ecc.vectortile.VectorTileDecoder;
import org.locationtech.jts.geom.GeometryFactory;
import org.locationtech.jts.geom.PrecisionModel;
import org.locationtech.jts.geom.impl.PackedCoordinateSequenceFactory;

public class MvtUtils {
  private static final String ID_KEY = "id";

  public static MapboxVectorTile decodeMvt(Path mvtFilePath) throws IOException {
    var mvt = Files.readAllBytes(mvtFilePath);
    return decodeMvt(mvt);
  }

  public static MapboxVectorTile decodeMvt2(Path mvtFilePath) throws IOException {
    var mvt = Files.readAllBytes(mvtFilePath);
    return decodeMvt2(mvt);
  }

  /* Ids are not present in the features -> add the ids to the feature from different decoder */
  // TODO: refactor -> ids are now present in the features with the latest version
  private static MapboxVectorTile decodeMvt(byte[] mvtTile) throws IOException {
    VectorTileDecoder mvtDecoder = new VectorTileDecoder();
    mvtDecoder.setAutoScale(false);

    var tile = mvtDecoder.decode(mvtTile);
    var mvtFeatures = tile.asList();
    var layers = new ArrayList<Layer>();
    for (var layerName : tile.getLayerNames()) {
      var layerFeatures =
          mvtFeatures.stream()
              .filter(f -> f.getLayerName().equals(layerName))
              .collect(Collectors.toList());

      var features = new ArrayList<Feature>();
      var tileExtent = 0;

      var ids = getIds(mvtTile, layerName);
      var i = 0;
      for (var mvtFeature : layerFeatures) {
        var properties = new HashMap(mvtFeature.getAttributes());
        var id = 0l;
        if (properties.containsKey(ID_KEY)) {
          if (properties.get(ID_KEY) instanceof String) {
            /* Rename the id property as it is a reserved column name in the MLT format */
            properties.put("_" + ID_KEY, properties.get(ID_KEY));
            properties.remove(ID_KEY);
          } else if (properties.get(ID_KEY) instanceof Long) {
            /* Rename the id property as it is a reserved column name in the MLT format */
            id = Long.valueOf(properties.get(ID_KEY).toString());
            properties.remove(ID_KEY);
          } else {
            throw new RuntimeException(
                "Only a string and long datatype for the id in the properties supported.");
          }
        } else {
          id = ids.get(i++);
        }

        // TODO: also transform properties
        var geometry = mvtFeature.getGeometry();
        // TODO: quick and dirty -> implement generic
        var transformedProperties = transformNestedPropertyNames(properties);
        var feature = new Feature(id, geometry, transformedProperties);
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

  public static List<VectorTileDecoder.Feature> decodeMvtFast(byte[] mvtTile) throws IOException {
    VectorTileDecoder mvtDecoder = new VectorTileDecoder();
    mvtDecoder.setAutoScale(false);

    var decodedTile = mvtDecoder.decode(mvtTile);
    var features = decodedTile.asList();
    return features;
  }

  private static List<Long> getIds(byte[] mvtTile, String layerName) throws IOException {
    final PrecisionModel precisionModel = new PrecisionModel();
    final PackedCoordinateSequenceFactory coordinateSequenceFactory =
        new PackedCoordinateSequenceFactory(PackedCoordinateSequenceFactory.DOUBLE);
    var geometryFactory = new GeometryFactory(precisionModel, 0, coordinateSequenceFactory);
    var result =
        MvtReader.loadMvt(
            new ByteArrayInputStream(mvtTile),
            geometryFactory,
            new TagKeyValueMapConverter(true, ID_KEY));

    var layer =
        result.getLayers().stream().filter(f -> f.getName().equals(layerName)).findFirst().get();
    return layer.getGeometries().stream()
        .mapToLong(f -> (long) ((LinkedHashMap<String, Object>) f.getUserData()).get(ID_KEY))
        .boxed()
        .collect(Collectors.toList());
  }

  // TODO: combine decodeMvt and decodeMvt2 to get correct features
  /* Ids are present but geometries of type polygon can have missing vertices and can be wrong */
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
        var properties = ((LinkedHashMap<String, Object>) mvtFeature.getUserData());
        var id = (long) properties.get(ID_KEY);
        properties.remove(ID_KEY);
        // TODO: quick and dirty -> implement generic
        var transformedProperties = transformNestedPropertyNames(properties);
        var feature = new Feature(id, mvtFeature, transformedProperties);
        features.add(feature);
      }

      layers.add(new Layer(name, features, layer.getExtent()));
    }

    return new MapboxVectorTile(layers);
  }

  // TODO: combine decodeMvt and decodeMvt2 to get correct features
  /* Ids are present but geometries of type polygon can have missing vertices and can be wrong */
  public static Collection<JtsLayer> decodeMvt2Fast(byte[] mvtTile) throws IOException {
    final PrecisionModel precisionModel = new PrecisionModel();
    final PackedCoordinateSequenceFactory coordinateSequenceFactory =
        new PackedCoordinateSequenceFactory(PackedCoordinateSequenceFactory.DOUBLE);
    var geometryFactory = new GeometryFactory(precisionModel, 0, coordinateSequenceFactory);
    var result =
        MvtReader.loadMvt(
            new ByteArrayInputStream(mvtTile),
            geometryFactory,
            new TagKeyValueMapConverter(true, ID_KEY));
    return result.getLayers();
  }

  private static LinkedHashMap<String, Object> transformNestedPropertyNames(
      Map<String, Object> properties) {
    var transformedProperties = new LinkedHashMap<String, Object>();
    properties.forEach((k, v) -> transformedProperties.put(k.replace("_", ":"), v));
    return transformedProperties;
  }
}
