package org.maplibre.mlt.decoder;

import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.io.InputStream;
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

  private static MltTilesetMetadata.Column.Builder decodeType(int typeCode) {
    final var builder = MltTilesetMetadata.Column.newBuilder();
    if (0 <= typeCode && typeCode <= 19) {
      return MltTilesetMetadata.Column.newBuilder()
          .setNullable((typeCode & 1) == 0)
          .setScalarType(
              MltTilesetMetadata.ScalarColumn.newBuilder()
                  .setPhysicalType(
                      switch (typeCode) {
                        case 0, 1 -> MltTilesetMetadata.ScalarType.BOOLEAN;
                        case 2, 3 -> MltTilesetMetadata.ScalarType.INT_8;
                        case 4, 5 -> MltTilesetMetadata.ScalarType.UINT_8;
                        case 6, 7 -> MltTilesetMetadata.ScalarType.INT_32;
                        case 8, 9 -> MltTilesetMetadata.ScalarType.UINT_32;
                        case 10, 11 -> MltTilesetMetadata.ScalarType.INT_64;
                        case 12, 13 -> MltTilesetMetadata.ScalarType.UINT_64;
                        case 14, 15 -> MltTilesetMetadata.ScalarType.FLOAT;
                        case 16, 17 -> MltTilesetMetadata.ScalarType.DOUBLE;
                        case 18, 19 -> MltTilesetMetadata.ScalarType.STRING;
                        default -> {
                          // Should be impossible due to the containing `if`
                          throw new IllegalStateException("Unsupported Type");
                        }
                      }));
    } else if (20 <= typeCode && typeCode <= 23) {
      return builder
          .setNullable(typeCode < 22)
          .setName(ID_COLUMN_NAME)
          .setScalarType(
              MltTilesetMetadata.ScalarColumn.newBuilder()
                  .setLongID((typeCode & 1) == 0)
                  .setLogicalType(MltTilesetMetadata.LogicalScalarType.ID));
    } else if (24 == typeCode) {
      return builder
          .setNullable(false)
          .setName(GEOMETRY_COLUMN_NAME)
          .setComplexType(
              MltTilesetMetadata.ComplexColumn.newBuilder()
                  .setPhysicalType(MltTilesetMetadata.ComplexType.GEOMETRY));
    } else if (25 == typeCode) {
      return builder
          .setNullable(false)
          .setComplexType(
              MltTilesetMetadata.ComplexColumn.newBuilder()
                  .setPhysicalType(MltTilesetMetadata.ComplexType.STRUCT));
    } else {
      throw new IllegalStateException("Unsupported Type " + typeCode);
    }
  }

  private static Layer parseBasicMVTEquivalent(int tag, InputStream stream) throws IOException {
    final var metadata = parseEmbeddedMetadata(stream);
    final var tileExtent = DecodingUtils.decodeVarint(stream);
    return decodeMltLayer(stream.readAllBytes(), metadata, tileExtent);
  }

  /** Decode an MLT tile with embedded metadata * */
  public static MapLibreTile decodeMlTile(byte[] tileData) throws IOException {
    final var layers = new ArrayList<Layer>();
    try (final var stream = new ByteArrayInputStream(tileData)) {
      while (stream.available() > 0) {
        final var length = DecodingUtils.decodeVarint(stream);
        final var tag = DecodingUtils.decodeVarintWithLength(stream);
        if (tag.getLeft() == 1) {
          final var layer = parseBasicMVTEquivalent(tag.getLeft(), stream);
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
    var offset = new IntWrapper(0);
    List<Long> ids = null;
    Geometry[] geometries = null;
    var properties = new HashMap<String, List<Object>>();
    for (var columnMetadata : layerMetadata.getColumnsList()) {
      final var columnName = columnMetadata.getName();
      final var numStreams = DecodingUtils.decodeVarint(tile, offset, 1)[0];
      // TODO: add decoding of vector type to be compliant with the spec
      // TODO: compare based on ids
      if (MltConverter.isID(columnMetadata)) {
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
        if (columnMetadata.getScalarType().getLongID()) {
          ids = IntegerDecoder.decodeLongStream(tile, offset, idDataStreamMetadata, false);
        } else {
          ids =
              IntegerDecoder.decodeIntStream(tile, offset, idDataStreamMetadata, false).stream()
                  .mapToLong(i -> i)
                  .boxed()
                  .collect(Collectors.toList());
        }
      } else if (MltConverter.isGeometry(columnMetadata)) {
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
    final var column = decodeType(typeCode);

    if (MltConverter.typeCodeHasName(typeCode)) {
      column.setName(DecodingUtils.decodeString(stream));
    }

    if (MltConverter.typeCodeHasChildren(typeCode)) {
      final var childCount = DecodingUtils.decodeVarint(stream);
      for (var i = 0; i < childCount; ++i) {
        column.setComplexType(
            MltTilesetMetadata.ComplexColumn.newBuilder(column.getComplexType())
                .addChildren(toField(decodeColumn(stream))));
      }
    }

    return column.build();
  }

  private static MltTilesetMetadata.Field toField(MltTilesetMetadata.Column column) {
    var field =
        MltTilesetMetadata.Field.newBuilder()
            .setNullable(column.getNullable())
            .setName(column.getName());
    if (column.hasScalarType()) {
      final var builder = MltTilesetMetadata.ScalarField.newBuilder();
      if (column.getScalarType().hasPhysicalType()) {
        field.setScalarField(builder.setPhysicalType(column.getScalarType().getPhysicalType()));
      } else if (column.getScalarType().hasLogicalType()) {
        field.setScalarField(builder.setLogicalType(column.getScalarType().getLogicalType()));
      } else {
        throw new RuntimeException("Unsupported Field Type");
      }
    } else if (column.hasComplexType()) {
      final var builder = MltTilesetMetadata.ComplexField.newBuilder();
      if (column.getComplexType().hasPhysicalType()) {
        field.setComplexField(builder.setPhysicalType(column.getComplexType().getPhysicalType()));
      } else if (column.getComplexType().hasLogicalType()) {
        field.setComplexField(
            builder.setLogicalType(column.getComplexTypeOrBuilder().getLogicalType()));
      } else {
        throw new RuntimeException("Unsupported Field Type");
      }
    } else {
      throw new RuntimeException("Unsupported Field Type");
    }
    return field.build();
  }

  public static MltTilesetMetadata.FeatureTableSchema parseEmbeddedMetadata(InputStream stream)
      throws IOException {
    final var table = MltTilesetMetadata.FeatureTableSchema.newBuilder();
    table.setName(DecodingUtils.decodeString(stream));

    final var columnCount = DecodingUtils.decodeVarint(stream);
    for (int i = 0; i < columnCount; ++i) {
      table.addColumns(decodeColumn(stream));
    }
    return table.build();
  }
}
