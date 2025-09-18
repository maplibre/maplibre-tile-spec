package org.maplibre.mlt.decoder;

import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.util.*;
import java.util.stream.Collectors;
import me.lemire.integercompression.IntWrapper;
import org.locationtech.jts.geom.Geometry;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Layer;
import org.maplibre.mlt.data.MapLibreTile;
import org.maplibre.mlt.metadata.stream.StreamMetadataDecoder;
import org.maplibre.mlt.metadata.tileset.ColumnOptions;
import org.maplibre.mlt.metadata.tileset.FieldOptions;
import org.maplibre.mlt.metadata.tileset.MltTilesetMetadata;

public class MltDecoder {
  private static final String ID_COLUMN_NAME = "id";
  private static final String GEOMETRY_COLUMN_NAME = "geometry";

  private MltDecoder() {}

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
        final var tag = DecodingUtils.decodeVarint(stream);
        switch (tag) {
          case 1, 2:
            final var layer = parseBasicMVTEquivalent(tag, stream);
            if (layer != null) {
              layers.add(layer);
            }
            break;
          default:
            var ignored = stream.skip(length);
        }
      }
    }
    return new MapLibreTile(layers);
  }

  /** Decodes an MLT tile in a similar in-memory representation then MVT is using */
  public static Layer decodeMltLayer(
      byte[] tile, MltTilesetMetadata.TileSetMetadata tileMetadata, int tileExtent)
      throws IOException {
    var offset = new IntWrapper(0);
    List<Long> ids = null;
    Geometry[] geometries = null;
    var properties = new HashMap<String, List<Object>>();
    var metadata = tileMetadata.getFeatureTables(0);
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

    return (geometries != null)
        ? convertToLayer(ids, geometries, properties, metadata, tileExtent)
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

  private static void decodeField(InputStream stream, MltTilesetMetadata.Field.Builder field)
      throws IOException {
    field.setName(DecodingUtils.decodeString(stream));

    final var options = DecodingUtils.decodeVarint(stream);
    final boolean logical = ((options & FieldOptions.LOGICAL_TYPE) != 0);
    field.setNullable((options & FieldOptions.NULLABLE) != 0);

    final var type = DecodingUtils.decodeVarint(stream);
    if ((options & FieldOptions.LOGICAL_TYPE) != 0) {
      final var complexField = field.getComplexFieldBuilder();
      if (logical) {
        complexField.setLogicalTypeValue(type);
      } else {
        complexField.setPhysicalTypeValue(type);
      }
      if ((options & FieldOptions.HAS_CHILDREN) != 0) {
        final var childCount = DecodingUtils.decodeVarint(stream);
        for (int i = 0; i < childCount; ++i) {
          decodeField(stream, complexField.addChildrenBuilder());
        }
      }
    } else {
      final var scalarType = field.getScalarFieldBuilder();
      if (logical) {
        scalarType.setLogicalTypeValue(type);
      } else {
        scalarType.setPhysicalTypeValue(type);
      }
    }
  }

  private static void decodeColumn(
      InputStream stream, int options, MltTilesetMetadata.Column.Builder column)
      throws IOException {
    final boolean logical = ((options & ColumnOptions.LOGICAL_TYPE) != 0);
    final var type = DecodingUtils.decodeVarint(stream);
    if ((options & ColumnOptions.COMPLEX_TYPE) != 0) {
      final var complexType = column.getComplexTypeBuilder();
      if (logical) {
        complexType.setLogicalTypeValue(type);
      } else {
        complexType.setPhysicalTypeValue(type);
      }
      if ((options & ColumnOptions.HAS_CHILDREN) != 0) {
        final var childCount = DecodingUtils.decodeVarint(stream);
        for (int i = 0; i < childCount; ++i) {
          decodeField(stream, complexType.addChildrenBuilder());
        }
      }
    } else {
      final var scalarType = column.getScalarTypeBuilder();
      if (logical) {
        scalarType.setLogicalTypeValue(type);
      } else {
        scalarType.setPhysicalTypeValue(type);
      }
    }
  }

  public static MltTilesetMetadata.TileSetMetadata parseEmbeddedMetadata(InputStream stream)
      throws IOException {
    final var result = MltTilesetMetadata.TileSetMetadata.newBuilder();
    final var table = result.addFeatureTablesBuilder();
    table.setName(DecodingUtils.decodeString(stream));

    final var columnCount = DecodingUtils.decodeVarint(stream);
    for (int i = 0; i < columnCount; ++i) {
      final var columnOptions = DecodingUtils.decodeVarint(stream);
      final var vertexScope = (columnOptions & ColumnOptions.VERTEX_SCOPE) != 0;
      final var column = table.addColumnsBuilder();
      column.setName(DecodingUtils.decodeString(stream));
      column.setNullable((columnOptions & ColumnOptions.NULLABLE) != 0);
      column.setColumnScope(
          vertexScope
              ? MltTilesetMetadata.ColumnScope.VERTEX
              : MltTilesetMetadata.ColumnScope.FEATURE);
      decodeColumn(stream, columnOptions, column);
    }
    return result.build();
  }
}
