package org.maplibre.mlt.decoder;

import com.google.common.io.CountingInputStream;
import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.util.*;
import java.util.stream.Collectors;
import me.lemire.integercompression.IntWrapper;
import org.locationtech.jts.geom.Geometry;
import org.maplibre.mlt.converter.MltConverter;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Layer;
import org.maplibre.mlt.data.MapLibreTile;
import org.maplibre.mlt.metadata.stream.StreamMetadataDecoder;
import org.maplibre.mlt.metadata.tileset.MltTilesetMetadata;

public class MltDecoder {
  private static final String ID_COLUMN_NAME = "id";
  private static final String GEOMETRY_COLUMN_NAME = "geometry";

  private MltDecoder() {}

  /** Decode an MLT tile with embedded metadata * */
  public static MapLibreTile decodeMlTile(byte[] tileData) throws IOException {
    final var result = new MapLibreTile(new ArrayList<>());
    try (final var rawStream = new ByteArrayInputStream(tileData);
        final var stream = new CountingInputStream(rawStream)) {
      // Each layer group...
      while (stream.available() > 0) {
        // Decode the size header
        final var metadataSize = DecodingUtils.decodeVarint(stream);
        final var tileDataSize = DecodingUtils.decodeVarint(stream);
        if (metadataSize + tileDataSize > stream.available()) {
          throw new RuntimeException("Invalid tile size");
        }

        // Parse the metadata
        final var headerSize = (int) stream.getCount();
        MltTilesetMetadata.TileSetMetadata metadata = null;
        try (final var metadataStream =
            new ByteArrayInputStream(tileData, headerSize, metadataSize)) {
          metadata = MltConverter.parseEmbeddedMetadata(metadataStream);
        }

        // Decode the tile data
        final var tile = decodeMlTile(tileData, headerSize + metadataSize, tileDataSize, metadata);

        // Aggregate the resulting layers
        result.layers().addAll(tile.layers());

        var ignored = stream.skip(metadataSize + tileDataSize);
      }
    }
    return result;
  }

  /** Decodes an MLT tile in a similar in-memory representation then MVT is using */
  public static MapLibreTile decodeMlTile(
      byte[] tile, MltTilesetMetadata.TileSetMetadata tileMetadata) throws IOException {
    return decodeMlTile(tile, 0, tile.length, tileMetadata);
  }

  /** Decodes an MLT tile in a similar in-memory representation then MVT is using */
  public static MapLibreTile decodeMlTile(
      byte[] tile,
      int tileByteOffset,
      int tileByteLength,
      MltTilesetMetadata.TileSetMetadata tileMetadata)
      throws IOException {
    var offset = new IntWrapper(tileByteOffset);
    var mltLayers = new ArrayList<Layer>();
    while (offset.get() < tileByteLength) {
      List<Long> ids = null;
      Geometry[] geometries = null;
      var properties = new HashMap<String, List<Object>>();

      var version = tile[offset.get()];
      offset.increment();
      var infos = DecodingUtils.decodeVarint(tile, offset, 5);
      var featureTableId = infos[0];
      var featureTableBodySize = infos[1];
      var tileExtent = infos[2];
      var maxTileExtent = infos[3];
      var numFeatures = infos[4];

      var metadata = tileMetadata.getFeatureTables(featureTableId);
      for (var columnMetadata : metadata.getColumnsList()) {
        var columnName = columnMetadata.getName();
        var numStreams = DecodingUtils.decodeVarint(tile, offset, 1)[0];
        // TODO: add decoding of vector type to be compliant with the spec
        // TODO: compare based on ids
        if (columnName.equals(ID_COLUMN_NAME)) {
          if (numStreams == 2) {
            var presentStreamMetadata = StreamMetadataDecoder.decode(tile, offset);
            // TODO: handle present stream -> should a id column even be nullable?
            var presentStream =
                DecodingUtils.decodeBooleanRle(
                    tile,
                    presentStreamMetadata.numValues(),
                    presentStreamMetadata.byteLength(),
                    offset);
          }

          var idDataStreamMetadata = StreamMetadataDecoder.decode(tile, offset);
          var idDataType = columnMetadata.getScalarType().getPhysicalType();
          if (idDataType.equals(MltTilesetMetadata.ScalarType.UINT_32)) {
            ids =
                IntegerDecoder.decodeIntStream(tile, offset, idDataStreamMetadata, false).stream()
                    .mapToLong(i -> i)
                    .boxed()
                    .collect(Collectors.toList());
          } else {
            ids = IntegerDecoder.decodeLongStream(tile, offset, idDataStreamMetadata, false);
          }
        } else if (columnName.equals(GEOMETRY_COLUMN_NAME)) {
          var geometryColumn = GeometryDecoder.decodeGeometryColumn(tile, numStreams, offset);
          geometries = GeometryDecoder.decodeGeometry(geometryColumn);
        } else {
          var propertyColumn =
              PropertyDecoder.decodePropertyColumn(tile, offset, columnMetadata, numStreams);
          if (propertyColumn instanceof HashMap<?, ?>) {
            @SuppressWarnings("unchecked")
            var p = ((Map<String, Object>) propertyColumn);
            for (var a : p.entrySet()) {
              if (a instanceof List<?>) {
                @SuppressWarnings("unchecked")
                var list = (List<Object>) a.getValue();
                properties.put(a.getKey(), list);
              }
            }
          } else if (propertyColumn instanceof List<?>) {
            @SuppressWarnings("unchecked")
            var list = (List<Object>) propertyColumn;
            properties.put(columnName, list);
          }
        }
      }

      if (geometries != null) {
        var layer = convertToLayer(ids, geometries, properties, metadata, numFeatures);
        mltLayers.add(layer);
      }
    }

    return new MapLibreTile(mltLayers);
  }

  private static Layer convertToLayer(
      List<Long> ids,
      Geometry[] geometries,
      Map<String, List<Object>> properties,
      MltTilesetMetadata.FeatureTableSchema metadata,
      int numFeatures) {
    if (numFeatures != geometries.length || numFeatures != ids.size()) {
      System.out.println(
          "Warning, in convertToLayer the size of ids("
              + ids.size()
              + "), geometries("
              + geometries.length
              + "), and features("
              + numFeatures
              + ") are not equal for layer: "
              + metadata.getName());
    }
    var features = new ArrayList<Feature>(numFeatures);
    for (var j = 0; j < numFeatures; j++) {
      var p = new HashMap<String, Object>();
      for (var propertyColumn : properties.entrySet()) {
        if (propertyColumn.getValue() == null) {
          p.put(propertyColumn.getKey(), null);
        } else {
          var v = propertyColumn.getValue().get(j);
          p.put(propertyColumn.getKey(), v);
        }
      }
      var feature = new Feature(ids.get(j), geometries[j], p);
      features.add(feature);
    }

    return new Layer(metadata.getName(), features, 0);
  }
}
