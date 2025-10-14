package org.maplibre.mlt.converter.mvt;

import io.github.sebasbaumh.mapbox.vectortile.adapt.jts.MvtReader;
import io.github.sebasbaumh.mapbox.vectortile.adapt.jts.TagKeyValueMapConverter;
import io.github.sebasbaumh.mapbox.vectortile.adapt.jts.model.JtsMvt;
import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.*;
import org.locationtech.jts.geom.GeometryFactory;
import org.locationtech.jts.geom.PrecisionModel;
import org.locationtech.jts.geom.impl.PackedCoordinateSequenceFactory;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Layer;

public class MvtTestUtils {
  private static final String ID_KEY = "id";

  /* Uses the mapbox-vector-tile-java library for decoding the MVT tile */
  // TODO: combine decodeMvt and decodeMvt2 to get correct features
  // Ids are present but geometries of type polygon can have missing vertices and can be wrong in
  // some cases
  public static MapboxVectorTile decodeMvt2(Path mvtFilePath) throws IOException {
    var mvt = Files.readAllBytes(mvtFilePath);
    return decodeMvt2(mvt);
  }

  /* Uses the mapbox-vector-tile-java library for decoding the MVT tile */
  public static JtsMvt decodeMvt2Fast(ByteArrayInputStream mvtTile) throws IOException {
    final PrecisionModel precisionModel = new PrecisionModel();
    final PackedCoordinateSequenceFactory coordinateSequenceFactory =
        new PackedCoordinateSequenceFactory(PackedCoordinateSequenceFactory.DOUBLE);
    var geometryFactory = new GeometryFactory(precisionModel, 0, coordinateSequenceFactory);
    return MvtReader.loadMvt(mvtTile, geometryFactory, new TagKeyValueMapConverter(true, ID_KEY));
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
        var transformedProperties = MvtUtils.transformNestedPropertyNames(properties, List.of());
        var feature = new Feature(id, mvtFeature, transformedProperties);
        features.add(feature);
      }

      layers.add(new Layer(name, features, layer.getExtent()));
    }

    return new MapboxVectorTile(layers);
  }
}
