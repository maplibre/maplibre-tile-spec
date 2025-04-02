package com.mlt.decoder.vectorized;

import com.mlt.metadata.stream.DictionaryType;
import com.mlt.metadata.stream.MortonEncodedStreamMetadata;
import com.mlt.metadata.stream.StreamMetadataDecoder;
import com.mlt.vector.VectorType;
import com.mlt.vector.geometry.GeometryVector;
import com.mlt.vector.geometry.TopologyVector;
import java.nio.IntBuffer;
import java.util.Optional;
import me.lemire.integercompression.IntWrapper;
import org.apache.commons.lang3.NotImplementedException;

public class VectorizedGeometryDecoder {
  public record GeometryColumn(
      IntBuffer geometryTypes,
      IntBuffer numGeometries,
      IntBuffer numParts,
      IntBuffer numRings,
      IntBuffer vertexOffsets,
      IntBuffer vertexBuffer,
      Optional<GeometryVector.MortonSettings> mortonSettings) {}

  private VectorizedGeometryDecoder() {}

  public static VectorizedGeometryDecoder.GeometryColumn decodeGeometryColumn(
      byte[] tile, int numStreams, IntWrapper offset) {
    var geometryTypeMetadata = StreamMetadataDecoder.decode(tile, offset);
    // TODO: use byte rle encoding
    var geometryTypes =
        VectorizedIntegerDecoder.decodeIntStream(tile, offset, geometryTypeMetadata, false);

    IntBuffer numGeometries = null;
    IntBuffer numParts = null;
    IntBuffer numRings = null;
    IntBuffer vertexOffsets = null;
    IntBuffer vertexBuffer = null;
    Optional<GeometryVector.MortonSettings> mortonSettings = Optional.empty();
    for (var i = 0; i < numStreams - 1; i++) {
      var geometryStreamMetadata = StreamMetadataDecoder.decode(tile, offset);
      switch (geometryStreamMetadata.physicalStreamType()) {
        case LENGTH:
          switch (geometryStreamMetadata.logicalStreamType().lengthType()) {
            case GEOMETRIES:
              numGeometries =
                  VectorizedIntegerDecoder.decodeIntStream(
                      tile, offset, geometryStreamMetadata, false);
              break;
            case PARTS:
              numParts =
                  VectorizedIntegerDecoder.decodeIntStream(
                      tile, offset, geometryStreamMetadata, false);
              break;
            case RINGS:
              numRings =
                  VectorizedIntegerDecoder.decodeIntStream(
                      tile, offset, geometryStreamMetadata, false);
              break;
            case TRIANGLES:
              throw new NotImplementedException("Not implemented yet.");
          }
          break;
        case OFFSET:
          vertexOffsets =
              VectorizedIntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, false);
          break;
        case DATA:
          if (DictionaryType.VERTEX.equals(
              geometryStreamMetadata.logicalStreamType().dictionaryType())) {
            vertexBuffer =
                VectorizedIntegerDecoder.decodeIntStream(
                    tile, offset, geometryStreamMetadata, true);
          } else {
            var mortonMetadata = (MortonEncodedStreamMetadata) geometryStreamMetadata;
            mortonSettings =
                Optional.of(
                    new GeometryVector.MortonSettings(
                        mortonMetadata.numBits(), mortonMetadata.coordinateShift()));
            vertexBuffer =
                VectorizedIntegerDecoder.decodeIntStream(
                    tile, offset, geometryStreamMetadata, false);
          }
          break;
      }
    }

    return new VectorizedGeometryDecoder.GeometryColumn(
        geometryTypes,
        numGeometries,
        numParts,
        numRings,
        vertexOffsets,
        vertexBuffer,
        mortonSettings);
  }

  // TODO: get rid of numFeatures parameter
  public static GeometryVector decodeToRandomAccessFormat(
      byte[] tile, int numStreams, IntWrapper offset, int numFeatures) {
    var geometryTypeMetadata = StreamMetadataDecoder.decode(tile, offset);
    var geometryTypesVectorType =
        VectorizedDecodingUtils.getVectorTypeIntStream(geometryTypeMetadata);

    IntBuffer numGeometries = null;
    IntBuffer numParts = null;
    IntBuffer numRings = null;
    IntBuffer vertexOffsets = null;
    IntBuffer vertexBuffer = null;
    Optional<GeometryVector.MortonSettings> mortonSettings = Optional.empty();

    if (geometryTypesVectorType.equals(VectorType.CONST)) {
      /* All geometries in the colum have the same geometry type */
      var geometryType =
          VectorizedIntegerDecoder.decodeConstIntStream(tile, offset, geometryTypeMetadata, false);

      for (var i = 0; i < numStreams - 1; i++) {
        var geometryStreamMetadata = StreamMetadataDecoder.decode(tile, offset);
        switch (geometryStreamMetadata.physicalStreamType()) {
          case LENGTH:
            switch (geometryStreamMetadata.logicalStreamType().lengthType()) {
              case GEOMETRIES:
                numGeometries =
                    VectorizedIntegerDecoder.decodeLengthStreamToOffsetBuffer(
                        tile, offset, geometryStreamMetadata);
                break;
              case PARTS:
                numParts =
                    VectorizedIntegerDecoder.decodeLengthStreamToOffsetBuffer(
                        tile, offset, geometryStreamMetadata);
                break;
              case RINGS:
                numRings =
                    VectorizedIntegerDecoder.decodeLengthStreamToOffsetBuffer(
                        tile, offset, geometryStreamMetadata);
                break;
              case TRIANGLES:
                throw new NotImplementedException("Not implemented yet.");
            }
            break;
          case OFFSET:
            vertexOffsets =
                VectorizedIntegerDecoder.decodeIntStream(
                    tile, offset, geometryStreamMetadata, false);
            break;
          case DATA:
            if (DictionaryType.VERTEX.equals(
                geometryStreamMetadata.logicalStreamType().dictionaryType())) {
              vertexBuffer =
                  VectorizedIntegerDecoder.decodeIntStream(
                      tile, offset, geometryStreamMetadata, true);
            } else {
              var mortonMetadata = (MortonEncodedStreamMetadata) geometryStreamMetadata;
              mortonSettings =
                  Optional.of(
                      new GeometryVector.MortonSettings(
                          mortonMetadata.numBits(), mortonMetadata.coordinateShift()));
              vertexBuffer =
                  VectorizedIntegerDecoder.decodeIntStream(
                      tile, offset, geometryStreamMetadata, false);
            }
            break;
        }
      }

      return mortonSettings.isPresent()
          ? GeometryVector.createConstMortonEncodedGeometryVector(
              numFeatures,
              geometryType,
              new TopologyVector(numGeometries, numParts, numRings),
              vertexOffsets,
              vertexBuffer,
              mortonSettings.get())
          :
          /* Currently only 2D coordinates (Vec2) are implemented in the encoder  */
          GeometryVector.createConst2DGeometryVector(
              numFeatures,
              geometryType,
              new TopologyVector(numGeometries, numParts, numRings),
              vertexOffsets,
              vertexBuffer);
    }

    /* Different geometry types are mixed in the geometry column */
    var geometryTypeVector =
        VectorizedIntegerDecoder.decodeIntStream(tile, offset, geometryTypeMetadata, false);

    for (var i = 0; i < numStreams - 1; i++) {
      var geometryStreamMetadata = StreamMetadataDecoder.decode(tile, offset);
      switch (geometryStreamMetadata.physicalStreamType()) {
        case LENGTH:
          switch (geometryStreamMetadata.logicalStreamType().lengthType()) {
            case GEOMETRIES:
              numGeometries =
                  VectorizedIntegerDecoder.decodeIntStream(
                      tile, offset, geometryStreamMetadata, false);
              break;
            case PARTS:
              numParts =
                  VectorizedIntegerDecoder.decodeIntStream(
                      tile, offset, geometryStreamMetadata, false);
              break;
            case RINGS:
              numRings =
                  VectorizedIntegerDecoder.decodeIntStream(
                      tile, offset, geometryStreamMetadata, false);
              break;
            case TRIANGLES:
              throw new NotImplementedException("Not implemented yet.");
          }
          break;
        case OFFSET:
          vertexOffsets =
              VectorizedIntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, false);
          break;
        case DATA:
          if (DictionaryType.VERTEX.equals(
              geometryStreamMetadata.logicalStreamType().dictionaryType())) {
            vertexBuffer =
                VectorizedIntegerDecoder.decodeIntStream(
                    tile, offset, geometryStreamMetadata, true);
          } else {
            var mortonMetadata = (MortonEncodedStreamMetadata) geometryStreamMetadata;
            mortonSettings =
                Optional.of(
                    new GeometryVector.MortonSettings(
                        mortonMetadata.numBits(), mortonMetadata.coordinateShift()));
            vertexBuffer =
                VectorizedIntegerDecoder.decodeIntStream(
                    tile, offset, geometryStreamMetadata, false);
          }
          break;
      }
    }

    // TODO: refactor the following instructions -> decode in one pass for performance reasons
    /* Calculate the offsets from the length buffer for random access */
    if (numGeometries != null) {
      numGeometries = decodeRootLengthStream(geometryTypeVector, numGeometries, 2);
      if (numParts != null && numRings != null) {
        numParts = decodeLevel1LengthStream(geometryTypeVector, numGeometries, numParts, false);
        numRings = decodeLevel2LengthStream(geometryTypeVector, numGeometries, numParts, numRings);
      } else if (numParts != null) {
        numParts =
            decodeLevel1WithoutRingBufferLengthStream(geometryTypeVector, numGeometries, numParts);
      }
    } else if (numParts != null && numRings != null) {
      numParts = decodeRootLengthStream(geometryTypeVector, numParts, 1);
      numRings = decodeLevel1LengthStream(geometryTypeVector, numParts, numRings, true);
    } else if (numParts != null) {
      numParts = decodeRootLengthStream(geometryTypeVector, numParts, 0);
    }

    return mortonSettings.isPresent()
        ? GeometryVector.createMortonEncodedGeometryVector(
            geometryTypeVector,
            new TopologyVector(numGeometries, numParts, numRings),
            vertexOffsets,
            vertexBuffer,
            mortonSettings.get())
        :
        /* Currently only 2D coordinates (Vec2) are implemented in the encoder  */
        GeometryVector.create2DGeometryVector(
            geometryTypeVector,
            new TopologyVector(numGeometries, numParts, numRings),
            vertexOffsets,
            vertexBuffer);
  }

  private static IntBuffer decodeGeometryLengthStream(
      IntBuffer geometryTypes, IntBuffer numGeometries) {
    var geometryOffsets = new int[geometryTypes.capacity() + 1];
    var previousOffset = 0;
    geometryOffsets[0] = previousOffset;
    var geometryCounter = 0;
    for (var i = 0; i < geometryTypes.capacity(); i++) {
      geometryOffsets[i + 1] =
          previousOffset + (geometryTypes.get(i) > 2 ? numGeometries.get(geometryCounter++) : 1);
      previousOffset = geometryOffsets[i + 1];
    }

    return IntBuffer.wrap(geometryOffsets);
  }

  /**
   * @param streamId 1 for numParts buffer and 0 for numRings buffer
   */
  private static IntBuffer decodeTopologyLengthStream(
      IntBuffer geometryTypes,
      IntBuffer topologyLengthBuffer,
      int topologyOffsetsBufferSize,
      int streamId,
      IntBuffer geometryOffsetBuffer,
      IntBuffer previousTopologyBuffer) {
    // TODO: refactor -> create a more efficient solution as this quick and dirty implementation
    var topologyOffsetsBuffer = new int[topologyOffsetsBufferSize + 1];
    topologyOffsetsBuffer[0] = 0;
    var previousTopologyBufferCounter = 1;
    var topologyLengthBufferCounter = 0;
    var topologyOffsetsBufferCounter = 1;
    for (var i = 1; i < geometryTypes.capacity(); i++) {
      var geometryType = geometryTypes.get(i - 1);
      // var previousOffset = i > 0? topologyOffsetsBuffer[i-1] : topologyOffsetsBuffer[i];
      var previousOffset = topologyOffsetsBuffer[topologyOffsetsBufferCounter - 1];
      if (geometryType <= 2) {
        /* Handle single part geometry types -> Point, LineString, Polygon
         *  case1: value exists in specific topology buffer (PartOffsets or RingsOffsets)
         *  -> always the case for Polygons and can be the case for LineStrings
         *  case2: There is no value in the current topology stream
         *  -> for example for Point geometry or LineString when current stream is PartOffsets
         *  and an additional RingOffsets stream is present
         * */
        if (previousTopologyBuffer != null) {
          var numParts = previousTopologyBuffer.get(previousTopologyBufferCounter++);
          for (var j = 0; j < numParts; j++) {
            topologyOffsetsBuffer[topologyOffsetsBufferCounter++] =
                previousOffset + topologyLengthBuffer.get(topologyLengthBufferCounter++);
          }
        } else {
          topologyOffsetsBuffer[topologyOffsetsBufferCounter++] =
              previousOffset
                  + ((geometryType > streamId)
                      ? topologyLengthBuffer.get(topologyLengthBufferCounter++)
                      : 1);
        }
      } else {

        /* Handle multipart geometry -> MultiPoint, MultiLineString, MultiPolygon */
        if (geometryType - 3 > streamId) {
          /* value exists in specific topology stream (PartOffsets or RingsOffsets)
           *  -> always the case for Polygons and can be the case for LineStrings */
          // numGeometries -> numParts -> numRings
          // 2 (polygons) -> 2, 2 (LinearRings) -> 4 , 5 | 6, 2 (Vertices)
          var numGeometries = geometryOffsetBuffer.get(i) - geometryOffsetBuffer.get(i - 1);
          if (previousTopologyBuffer != null) {
            /* Handle ringOffsets buffer case */
            for (var j = 0; j <= numGeometries; j++) {
              /* Get the number of LinearRings per geometry */
              var numParts =
                  previousTopologyBuffer.get(previousTopologyBufferCounter)
                      - previousTopologyBuffer.get(previousTopologyBufferCounter - 1);
              previousTopologyBufferCounter++;
              for (var k = 0; k < numParts; k++) {
                /* Add the number of vertices to the ringOffsets buffer  */
                topologyOffsetsBuffer[topologyOffsetsBufferCounter++] =
                    previousOffset + topologyLengthBuffer.get(topologyLengthBufferCounter++);
              }
            }
          } else {
            /* Handle partOffsets buffer case */
            for (var j = 0; j <= numGeometries; j++) {
              /* Get the number of LinearRings per geometry */
              var numParts =
                  previousTopologyBuffer.get(previousTopologyBufferCounter)
                      - previousTopologyBuffer.get(previousTopologyBufferCounter - 1);
              previousTopologyBufferCounter++;
              for (var k = 0; k < numParts; k++) {
                /* Add the number of vertices to the ringOffsets buffer  */
                topologyOffsetsBuffer[topologyOffsetsBufferCounter++] =
                    previousOffset + topologyLengthBuffer.get(topologyLengthBufferCounter++);
              }
            }
          }

          var numParts =
              previousTopologyBuffer.get(previousTopologyBufferCounter)
                  - previousTopologyBuffer.get(previousTopologyBufferCounter - 1);
          previousTopologyBufferCounter++;
          // TODO: iterate as of as number of previous stream
          for (var j = 1; j <= numParts; j++) {
            topologyOffsetsBuffer[i] = previousOffset + topologyLengthBuffer.get(i);
          }
        } else {
          /* There is no value in the current topology stream but a parent stream
           * has a value for this geometry e.g. partOffsets and ringOffsets streams
           * for mixed geometryTypes of MultiPolygon and MultiPoint.
           * Take the value from geometryOffsetsBuffer and repeat as many times as the value.
           * For example a MultiPolygon and MultiPoint geometry are mixed in column.
           * When the MultiPoint geometry consists of 5 points then add 5 times one to the PartOffset and
           * RingOffset stream to enable random access.
           * */
          var numGeometries = geometryOffsetBuffer.get(i) - geometryOffsetBuffer.get(i - 1);
          for (var j = 1; j <= numGeometries; j++) {
            topologyOffsetsBuffer[i] = previousOffset + j;
          }
        }
      }
    }

    return IntBuffer.wrap(topologyOffsetsBuffer);
  }

  /**
   * Handle the parsing of the different topology length buffers separate not generic to reduce the
   * branching and improve the performance
   */
  private static IntBuffer decodeRootLengthStream(
      IntBuffer geometryTypes, IntBuffer rootLengthStream, int bufferId) {
    var rootBufferOffsets = new int[geometryTypes.capacity() + 1];
    var previousOffset = 0;
    rootBufferOffsets[0] = previousOffset;
    var rootLengthCounter = 0;
    for (var i = 0; i < geometryTypes.capacity(); i++) {
      /* Test if the geometry has and entry in the root buffer
       * BufferId: 2 GeometryOffsets -> MultiPolygon, MultiLineString, MultiPoint
       * BufferId: 1 PartOffsets -> Polygon
       * BufferId: 0 PartOffsets, RingOffsets -> LineString
       * */
      previousOffset =
          rootBufferOffsets[i + 1] =
              previousOffset
                  + (geometryTypes.get(i) > bufferId
                      ? rootLengthStream.get(rootLengthCounter++)
                      : 1);
    }

    return IntBuffer.wrap(rootBufferOffsets);
  }

  private static IntBuffer decodeLevel1LengthStream(
      IntBuffer geometryTypes,
      IntBuffer rootOffsetBuffer,
      IntBuffer level1LengthBuffer,
      boolean isLineStringPresent) {
    var level1BufferOffsets = new int[rootOffsetBuffer.get(rootOffsetBuffer.capacity() - 1) + 1];
    var previousOffset = 0;
    level1BufferOffsets[0] = previousOffset;
    var level1BufferCounter = 1;
    var level1LengthBufferCounter = 0;
    for (var i = 0; i < geometryTypes.capacity(); i++) {
      var geometryType = geometryTypes.get(i);
      var numGeometries = rootOffsetBuffer.get(i + 1) - rootOffsetBuffer.get(i);
      if (geometryType == 5
          || geometryType == 2
          || (isLineStringPresent && (geometryType == 4 || geometryType == 1))) {
        /* For MultiPolygon, Polygon and in some cases for MultiLineString and LineString
         * a value in the level1LengthBuffer exists */
        for (var j = 0; j < numGeometries; j++) {
          previousOffset =
              level1BufferOffsets[level1BufferCounter++] =
                  previousOffset + level1LengthBuffer.get(level1LengthBufferCounter++);
        }
      } else {
        /* For MultiPoint and Point and in some cases for MultiLineString and LineString no value in the
         * level1LengthBuffer exists */
        for (var j = 0; j < numGeometries; j++) {
          level1BufferOffsets[level1BufferCounter++] = ++previousOffset;
        }
      }
    }

    return IntBuffer.wrap(level1BufferOffsets);
  }

  /**
   * Case where no ring buffer exists so no MultiPolygon or Polygon geometry is part of the buffer
   */
  private static IntBuffer decodeLevel1WithoutRingBufferLengthStream(
      IntBuffer geometryTypes, IntBuffer rootOffsetBuffer, IntBuffer level1LengthBuffer) {
    var level1BufferOffsets = new int[rootOffsetBuffer.get(rootOffsetBuffer.capacity() - 1) + 1];
    var previousOffset = 0;
    level1BufferOffsets[0] = previousOffset;
    var level1OffsetBufferCounter = 1;
    var level1LengthCounter = 0;
    for (var i = 0; i < geometryTypes.capacity(); i++) {
      var geometryType = geometryTypes.get(i);
      var numGeometries = rootOffsetBuffer.get(i + 1) - rootOffsetBuffer.get(i);
      if (geometryType == 4 || geometryType == 1) {
        /* For MultiLineString and LineString a value in the level1LengthBuffer exists */
        for (var j = 0; j < numGeometries; j++) {
          previousOffset =
              level1BufferOffsets[level1OffsetBufferCounter++] =
                  previousOffset + level1LengthBuffer.get(level1LengthCounter++);
        }
      } else {
        /* For MultiPoint and Point no value in level1LengthBuffer exists */
        for (var j = 0; j < numGeometries; j++) {
          level1BufferOffsets[level1OffsetBufferCounter++] = ++previousOffset;
        }
      }
    }

    return IntBuffer.wrap(level1BufferOffsets);
  }

  private static IntBuffer decodeLevel2LengthStream(
      IntBuffer geometryTypes,
      IntBuffer rootOffsetBuffer,
      IntBuffer level1OffsetBuffer,
      IntBuffer level2LengthBuffer) {
    var level2BufferOffsets =
        new int[level1OffsetBuffer.get(level1OffsetBuffer.capacity() - 1) + 1];
    var previousOffset = 0;
    level2BufferOffsets[0] = previousOffset;
    var level1OffsetBufferCounter = 1;
    var level2OffsetBufferCounter = 1;
    var level2LengthBufferCounter = 0;
    for (var i = 0; i < geometryTypes.capacity(); i++) {
      var geometryType = geometryTypes.get(i);
      var numGeometries = rootOffsetBuffer.get(i + 1) - rootOffsetBuffer.get(i);
      if (geometryType != 0 && geometryType != 3) {
        /* For MultiPolygon, MultiLineString, Polygon and LineString a value in level2LengthBuffer
         * exists */
        for (var j = 0; j < numGeometries; j++) {
          var numParts =
              level1OffsetBuffer.get(level1OffsetBufferCounter)
                  - level1OffsetBuffer.get(level1OffsetBufferCounter - 1);
          level1OffsetBufferCounter++;
          for (var k = 0; k < numParts; k++) {
            previousOffset =
                level2BufferOffsets[level2OffsetBufferCounter++] =
                    previousOffset + level2LengthBuffer.get(level2LengthBufferCounter++);
          }
        }
      } else {
        /* For MultiPoint and Point no value in level2LengthBuffer exists */
        for (var j = 0; j < numGeometries; j++) {
          level2BufferOffsets[level2OffsetBufferCounter++] = ++previousOffset;
          level1OffsetBufferCounter++;
        }
      }
    }

    return IntBuffer.wrap(level2BufferOffsets);
  }
}
