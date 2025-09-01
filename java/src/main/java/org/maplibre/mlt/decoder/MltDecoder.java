package org.maplibre.mlt.decoder;

import java.io.IOException;
import java.util.*;
import java.util.stream.Collectors;
import me.lemire.integercompression.IntWrapper;
import org.locationtech.jts.geom.Geometry;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Layer;
import org.maplibre.mlt.data.MapLibreTile;
import org.maplibre.mlt.decoder.vectorized.VectorizedDecodingUtils;
import org.maplibre.mlt.decoder.vectorized.VectorizedGeometryDecoder;
import org.maplibre.mlt.decoder.vectorized.VectorizedIntegerDecoder;
import org.maplibre.mlt.decoder.vectorized.VectorizedPropertyDecoder;
import org.maplibre.mlt.metadata.stream.RleEncodedStreamMetadata;
import org.maplibre.mlt.metadata.stream.StreamMetadataDecoder;
import org.maplibre.mlt.metadata.tileset.MltTilesetMetadata;
import org.maplibre.mlt.vector.BitVector;
import org.maplibre.mlt.vector.FeatureTable;
import org.maplibre.mlt.vector.Vector;
import org.maplibre.mlt.vector.VectorType;
import org.maplibre.mlt.vector.constant.IntConstVector;
import org.maplibre.mlt.vector.constant.LongConstVector;
import org.maplibre.mlt.vector.flat.IntFlatVector;
import org.maplibre.mlt.vector.flat.LongFlatVector;
import org.maplibre.mlt.vector.geometry.GeometryVector;
import org.maplibre.mlt.vector.sequence.IntSequenceVector;
import org.maplibre.mlt.vector.sequence.LongSequenceVector;

public class MltDecoder {
  private static final String ID_COLUMN_NAME = "id";
  private static final String GEOMETRY_COLUMN_NAME = "geometry";

  private MltDecoder() {}

  /** Decodes an MLT tile in a similar in-memory representation then MVT is using */
  public static MapLibreTile decodeMlTile(
      byte[] tile, MltTilesetMetadata.TileSetMetadata tileMetadata) throws IOException {
    var offset = new IntWrapper(0);
    var mltLayers = new ArrayList<Layer>();
    while (offset.get() < tile.length) {
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
            var p = ((Map<String, Object>) propertyColumn);
            for (var a : p.entrySet()) {
              properties.put(a.getKey(), (List<Object>) a.getValue());
            }
          } else {
            properties.put(columnName, (ArrayList) propertyColumn);
          }
        }
      }

      var layer = convertToLayer(ids, geometries, properties, metadata, numFeatures);
      mltLayers.add(layer);
    }

    return new MapLibreTile(mltLayers);
  }

  /**
   * Converts a tile from the MLT storage into the in-memory format, which should be the preferred
   * way for processing the data in the future. The in-memory format is optimized for random access.
   * Currently, the decoding is not fully utilizing vectorized instructions (SIMD). But the goal is
   * to fully exploit this kind of instruction in the next step.
   */
  public static FeatureTable[] decodeMlTileVectorized(
      byte[] tile, MltTilesetMetadata.TileSetMetadata tileMetadata) {
    var offset = new IntWrapper(0);
    var featureTables = new FeatureTable[tileMetadata.getFeatureTablesCount()];
    while (offset.get() < tile.length) {
      var version = tile[offset.get()];
      offset.increment();
      var infos = DecodingUtils.decodeVarint(tile, offset, 5);
      var featureTableId = infos[0];
      var featureTableBodySize = infos[1];
      var tileExtent = infos[2];
      var maxTileExtent = infos[3];
      var numFeatures = infos[4];
      var metadata = tileMetadata.getFeatureTables(featureTableId);

      Vector idVector = null;
      GeometryVector geometryVector = null;
      var propertyVectors = new ArrayList<Vector>();
      var columList = metadata.getColumnsList();
      for (var columnMetadata : columList) {
        var columnName = columnMetadata.getName();
        var numStreams = DecodingUtils.decodeVarint(tile, offset, 1)[0];

        // TODO: add decoding of vector type to be compliant with the spec
        if (columnName.equals(ID_COLUMN_NAME)) {
          BitVector nullabilityBuffer = null;
          if (numStreams == 2) {
            var presentStreamMetadata = StreamMetadataDecoder.decode(tile, offset);
            var values =
                VectorizedDecodingUtils.decodeBooleanRle(
                    tile, presentStreamMetadata.numValues(), offset);
            nullabilityBuffer = new BitVector(values, presentStreamMetadata.numValues());
          }

          idVector = decodeIdColumn(tile, columnMetadata, offset, columnName, nullabilityBuffer);
        } else if (columnName.equals(GEOMETRY_COLUMN_NAME)) {
          geometryVector =
              VectorizedGeometryDecoder.decodeToRandomAccessFormat(
                  tile, numStreams, offset, numFeatures);
        } else {
          if (numStreams == 0 && columnMetadata.hasScalarType()) {
            continue;
          }

          var propertyVector =
              VectorizedPropertyDecoder.decodeToRandomAccessFormat(
                  tile, offset, columnMetadata, numStreams, numFeatures);
          if (propertyVector != null) {
            propertyVectors.add(propertyVector);
          }
        }
      }

      var featureTable =
          new FeatureTable(
              metadata.getName(), idVector, geometryVector, propertyVectors.toArray(new Vector[0]));
      featureTables[featureTableId] = featureTable;
    }

    return featureTables;
  }

  private static Vector decodeIdColumn(
      byte[] tile,
      MltTilesetMetadata.Column columnMetadata,
      IntWrapper offset,
      String columnName,
      BitVector nullabilityBuffer) {
    /* If an id column is present the column is not allowed to be nullable */
    var idDataStreamMetadata = StreamMetadataDecoder.decode(tile, offset);
    var idDataType = columnMetadata.getScalarType().getPhysicalType();
    var vectorType = VectorizedDecodingUtils.getVectorTypeIntStream(idDataStreamMetadata);
    if (idDataType.equals(MltTilesetMetadata.ScalarType.UINT_32)) {
      // TODO: add support for const vector type -> but should not be allowed in id column
      if (vectorType.equals(VectorType.FLAT)) {
        var id =
            VectorizedIntegerDecoder.decodeIntStream(tile, offset, idDataStreamMetadata, false);
        return new IntFlatVector(columnName, nullabilityBuffer, id);
      } else if (vectorType.equals(VectorType.CONST)) {
        var id =
            VectorizedIntegerDecoder.decodeConstIntStream(
                tile, offset, idDataStreamMetadata, false);
        return new IntConstVector(columnName, nullabilityBuffer, id);
      } else if (vectorType.equals(VectorType.SEQUENCE)) {
        var id =
            VectorizedIntegerDecoder.decodeSequenceIntStream(tile, offset, idDataStreamMetadata);
        return new IntSequenceVector(
            columnName,
            id.getLeft(),
            id.getRight(),
            ((RleEncodedStreamMetadata) idDataStreamMetadata).numRleValues());
      } else {
        throw new IllegalArgumentException("Vector type not supported for id column.");
      }
    } else {
      // TODO: add support for const vector type -> but should not be allowed in id column
      if (vectorType.equals(VectorType.FLAT)) {
        var id =
            VectorizedIntegerDecoder.decodeLongStream(tile, offset, idDataStreamMetadata, false);
        return new LongFlatVector(columnName, nullabilityBuffer, id);
      } else if (vectorType.equals(VectorType.CONST)) {
        var id =
            VectorizedIntegerDecoder.decodeConstLongStream(
                tile, offset, idDataStreamMetadata, false);
        return new LongConstVector(columnName, nullabilityBuffer, id);
      } else if (vectorType.equals(VectorType.SEQUENCE)) {
        var id =
            VectorizedIntegerDecoder.decodeSequenceLongStream(tile, offset, idDataStreamMetadata);
        return new LongSequenceVector(
            columnName,
            id.getLeft(),
            id.getRight(),
            ((RleEncodedStreamMetadata) idDataStreamMetadata).numRleValues());
      } else {
        throw new IllegalArgumentException("Vector type not supported for id column.");
      }
    }
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
