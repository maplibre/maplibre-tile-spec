package org.maplibre.mlt.decoder;

import com.google.common.io.CountingInputStream;
import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.util.*;
import java.util.stream.Collectors;
import me.lemire.integercompression.IntWrapper;
import org.apache.commons.lang3.tuple.Pair;
import org.locationtech.jts.geom.Geometry;
import org.maplibre.mlt.converter.encodings.MltTypeMap;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Layer;
import org.maplibre.mlt.data.MapLibreTile;
import org.maplibre.mlt.metadata.stream.StreamMetadataDecoder;
import org.maplibre.mlt.metadata.tileset.MltTilesetMetadata;

public class MltDecoder {
  private MltDecoder() {}

  private static Layer parseBasicMVTEquivalent(int layerSize, InputStream stream)
      throws IOException {
    try (var countStream = new CountingInputStream(stream)) {
      final var metadataExtent = parseEmbeddedMetadata(countStream);
      final var metadata = metadataExtent.getLeft();
      final var tileExtent = metadataExtent.getRight();
      final var bodySize = layerSize - countStream.getCount();
      return decodeMltLayer(countStream.readNBytes((int) bodySize), metadata, tileExtent);
    }
  }

  /** Decode an MLT tile with embedded metadata * */
  public static MapLibreTile decodeMlTile(byte[] tileData) throws IOException {
    final var layers = new ArrayList<Layer>();
    try (final var stream = new ByteArrayInputStream(tileData)) {
      while (stream.available() > 0) {
        final var length = DecodingUtils.decodeVarint(stream);
        final var tag = DecodingUtils.decodeVarintWithLength(stream);
        final var bodySize = length - tag.getRight();
        if (tag.getLeft() == 1) {
          final var layer = parseBasicMVTEquivalent(bodySize, stream);
          if (layer != null) {
            layers.add(layer);
          }
        } else {
          // Skip the remainder of this one
          final var ignored = stream.skip(length - tag.getRight());
        }
      }
    }
    return new MapLibreTile(layers);
  }

  /** Decodes an MLT tile in a similar in-memory representation then MVT is using */
  public static Layer decodeMltLayer(
      byte[] tile, MltTilesetMetadata.FeatureTableSchema layerMetadata, int tileExtent)
      throws IOException {
    final var offset = new IntWrapper(0);
    List<Long> ids = null;
    Geometry[] geometries = null;
    final var properties = new HashMap<String, List<Object>>();
    for (var columnMetadata : layerMetadata.getColumnsList()) {
      final var columnName = columnMetadata.getName();
      final var hasStreamCount = MltTypeMap.Tag0x01.hasStreamCount(columnMetadata);
      final var numStreams = hasStreamCount ? DecodingUtils.decodeVarints(tile, offset, 1)[0] : 0;
      // TODO: add decoding of vector type to be compliant with the spec
      // TODO: compare based on ids
      if (MltTypeMap.Tag0x01.isID(columnMetadata)) {
        if (columnMetadata.getNullable()) {
          final var presentStreamMetadata = StreamMetadataDecoder.decode(tile, offset);
          // TODO: handle present stream -> should a id column even be nullable?
          offset.add(presentStreamMetadata.byteLength());
        }

        final var idDataStreamMetadata = StreamMetadataDecoder.decode(tile, offset);
        if (columnMetadata.getScalarType().getLongID()) {
          ids = IntegerDecoder.decodeLongStream(tile, offset, idDataStreamMetadata, false);
        } else {
          ids =
              IntegerDecoder.decodeIntStream(tile, offset, idDataStreamMetadata, false).stream()
                  .mapToLong(i -> i)
                  .boxed()
                  .collect(Collectors.toList());
        }
      } else if (MltTypeMap.Tag0x01.isGeometry(columnMetadata)) {
        assert hasStreamCount;
        final var geometryColumn = GeometryDecoder.decodeGeometryColumn(tile, numStreams, offset);
        geometries = GeometryDecoder.decodeGeometry(geometryColumn);
      } else {
        final var propertyColumn =
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
        } else {
          throw new RuntimeException("Unexpected property result");
        }
      }
    }

    return (geometries != null)
        ? convertToLayer(ids, geometries, properties, layerMetadata, tileExtent)
        : null;
  }

  private static Layer convertToLayer(
      List<Long> ids,
      Geometry[] geometries,
      Map<String, List<Object>> properties,
      MltTilesetMetadata.FeatureTableSchema metadata,
      int tileExtent) {
    if (ids != null && geometries.length != ids.size()) {
      System.out.println(
          "Warning, in convertToLayer the size of ids("
              + ids.size()
              + "), geometries("
              + geometries.length
              + "), are not equal for layer: "
              + metadata.getName());
    }
    var features = new ArrayList<Feature>(geometries.length);
    for (var j = 0; j < geometries.length; j++) {
      var p = new HashMap<String, Object>();
      for (var propertyColumn : properties.entrySet()) {
        if (propertyColumn.getValue() == null) {
          p.put(propertyColumn.getKey(), null);
        } else {
          var v = propertyColumn.getValue().get(j);
          p.put(propertyColumn.getKey(), v);
        }
      }
      final var id = (ids != null) ? ids.get(j) : 0;
      var feature = new Feature(id, geometries[j], p);
      features.add(feature);
    }

    return new Layer(metadata.getName(), features, tileExtent);
  }

  private static MltTilesetMetadata.Column decodeColumn(InputStream stream) throws IOException {
    final var typeCode = DecodingUtils.decodeVarint(stream);
    final var column = MltTypeMap.Tag0x01.decodeColumnType(typeCode);

    if (MltTypeMap.Tag0x01.columnTypeHasName(typeCode)) {
      column.setName(DecodingUtils.decodeString(stream));
    }

    if (MltTypeMap.Tag0x01.columnTypeHasChildren(typeCode)) {
      final var childCount = DecodingUtils.decodeVarint(stream);
      for (var i = 0; i < childCount; ++i) {
        column.setComplexType(
            MltTilesetMetadata.ComplexColumn.newBuilder(column.getComplexType())
                .addChildren(MltTypeMap.Tag0x01.toField(decodeColumn(stream))));
      }
    }

    return column.build();
  }

  public static Pair<MltTilesetMetadata.FeatureTableSchema, Integer> parseEmbeddedMetadata(
      InputStream stream) throws IOException {
    final var table = MltTilesetMetadata.FeatureTableSchema.newBuilder();
    table.setName(DecodingUtils.decodeString(stream));
    final var extent = DecodingUtils.decodeVarint(stream);

    final var columnCount = DecodingUtils.decodeVarint(stream);
    for (int i = 0; i < columnCount; ++i) {
      table.addColumns(decodeColumn(stream));
    }
    return Pair.of(table.build(), extent);
  }
}
