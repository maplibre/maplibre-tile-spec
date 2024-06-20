package com.mlt.decoder;

import com.mlt.data.Feature;
import com.mlt.data.Layer;
import com.mlt.data.MapLibreTile;
import com.mlt.decoder.vectorized.VectorizedDecodingUtils;
import com.mlt.decoder.vectorized.VectorizedGeometryDecoder;
import com.mlt.decoder.vectorized.VectorizedIntegerDecoder;
import com.mlt.decoder.vectorized.VectorizedPropertyDecoder;
import com.mlt.metadata.stream.StreamMetadataDecoder;
import com.mlt.metadata.tileset.MltTilesetMetadata;
import com.mlt.vector.BitVector;
import com.mlt.vector.FeatureTable;
import com.mlt.vector.Vector;
import com.mlt.vector.flat.IntFlatVector;
import com.mlt.vector.flat.LongFlatVector;
import com.mlt.vector.geometry.GeometryVector;
import java.io.IOException;
import java.util.*;
import java.util.stream.Collectors;
import me.lemire.integercompression.IntWrapper;
import org.locationtech.jts.geom.Geometry;

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
      var infos = DecodingUtils.decodeVarint(tile, offset, 4);
      var featureTableId = infos[0];
      var tileExtent = infos[1];
      var maxTileExtent = infos[2];
      var numFeatures = infos[3];

      var metadata = tileMetadata.getFeatureTables(featureTableId);
      for (var columnMetadata : metadata.getColumnsList()) {
        var columnName = columnMetadata.getName();
        var numStreams = DecodingUtils.decodeVarint(tile, offset, 1)[0];
        // TODO: add decoding of vector type to be compliant with the spec
        // TODO: compare based on ids
        if (columnName.equals(ID_COLUMN_NAME)) {
          if (numStreams == 2) {
            var presentStreamMetadata = StreamMetadataDecoder.decode(tile, offset);
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
      var infos = DecodingUtils.decodeVarint(tile, offset, 4);
      var featureTableId = infos[0];
      var tileExtent = infos[1];
      var maxTileExtent = infos[2];
      var numFeatures = infos[3];

      var propertyIndex = 0;
      var metadata = tileMetadata.getFeatureTables(featureTableId);

      Vector idVector = null;
      GeometryVector geometryVector = null;
      /* Id column always has to be the first column in a FeatureTable */
      var numProperties =
          metadata.getColumnsList().size()
              - (metadata.getColumnsList().get(0).getName().equals(ID_COLUMN_NAME) ? 2 : 1);
      var propertyVectors = new Vector[numProperties];
      for (var columnMetadata : metadata.getColumnsList()) {
        var columnName = columnMetadata.getName();
        var numStreams = DecodingUtils.decodeVarint(tile, offset, 1)[0];

        // TODO: add decoding of vector type to be compliant with the spec
        // TODO: compare based on ids
        if (columnName.equals(ID_COLUMN_NAME)) {
          BitVector nullabilityBuffer = null;
          if (numStreams == 2) {
            var presentStreamMetadata = StreamMetadataDecoder.decode(tile, offset);
            var n =
                VectorizedDecodingUtils.decodeBooleanRle(
                    tile, presentStreamMetadata.numValues(), offset);
            nullabilityBuffer = new BitVector(n, presentStreamMetadata.numValues());
          }

          // TODO: are ids optional? -> if not transform to random access format
          var idDataStreamMetadata = StreamMetadataDecoder.decode(tile, offset);
          var idDataType = columnMetadata.getScalarType().getPhysicalType();
          // TODO: check for const and sequence vectors to reduce decoding time
          if (idDataType.equals(MltTilesetMetadata.ScalarType.UINT_32)) {
            var id =
                VectorizedIntegerDecoder.decodeIntStream(tile, offset, idDataStreamMetadata, false);
            idVector = new IntFlatVector(columnName, nullabilityBuffer, id);
          } else {
            var id =
                VectorizedIntegerDecoder.decodeLongStream(
                    tile, offset, idDataStreamMetadata, false);
            idVector = new LongFlatVector(columnName, nullabilityBuffer, id);
          }
        } else if (columnName.equals(GEOMETRY_COLUMN_NAME)) {
          geometryVector =
              VectorizedGeometryDecoder.decodeToRandomAccessFormat(
                  tile, numStreams, offset, numFeatures);
        } else {
          var propertyVector =
              VectorizedPropertyDecoder.decodeToRandomAccessFormat(
                  tile, offset, columnMetadata, numStreams, numFeatures);
          propertyVectors[propertyIndex++] = propertyVector;
        }
      }

      var featureTable =
          new FeatureTable(metadata.getName(), idVector, geometryVector, propertyVectors);
      featureTables[featureTableId] = featureTable;
    }

    return featureTables;
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
