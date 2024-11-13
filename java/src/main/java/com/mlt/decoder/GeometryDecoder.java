package com.mlt.decoder;

import com.mlt.converter.geometry.GeometryType;
import com.mlt.converter.geometry.ZOrderCurve;
import com.mlt.metadata.stream.*;
import com.mlt.vector.geometry.GeometryVector;
import java.util.Arrays;
import java.util.List;
import java.util.stream.Collectors;
import me.lemire.integercompression.IntWrapper;
import org.apache.commons.lang3.NotImplementedException;
import org.locationtech.jts.geom.*;

public class GeometryDecoder {

  public record GeometryColumn(
      List<Integer> geometryTypes,
      List<Integer> numGeometries,
      List<Integer> numParts,
      List<Integer> numRings,
      List<Integer> vertexOffsets,
      List<Integer> vertexList) {}

  private GeometryDecoder() {}

  public static GeometryColumn decodeGeometryColumn(
      byte[] tile, int numStreams, IntWrapper offset) {
    var geometryTypeMetadata = StreamMetadataDecoder.decode(tile, offset);
    var geometryTypes = IntegerDecoder.decodeIntStream(tile, offset, geometryTypeMetadata, false);

    List<Integer> numGeometries = null;
    List<Integer> numParts = null;
    List<Integer> numRings = null;
    List<Integer> vertexOffsets = null;
    List<Integer> vertexList = null;
    for (var i = 0; i < numStreams - 1; i++) {
      var geometryStreamMetadata = StreamMetadataDecoder.decode(tile, offset);
      switch (geometryStreamMetadata.physicalStreamType()) {
        case LENGTH:
          switch (geometryStreamMetadata.logicalStreamType().lengthType()) {
            case GEOMETRIES:
              numGeometries =
                  IntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, false);
              break;
            case PARTS:
              numParts =
                  IntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, false);
              break;
            case RINGS:
              numRings =
                  IntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, false);
              break;
            case TRIANGLES:
              throw new NotImplementedException("Not implemented yet.");
          }
          break;
        case OFFSET:
          vertexOffsets =
              IntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, false);
          break;
        case DATA:
          if (DictionaryType.VERTEX.equals(
              geometryStreamMetadata.logicalStreamType().dictionaryType())) {
            if (geometryStreamMetadata.physicalLevelTechnique()
                == PhysicalLevelTechnique.FAST_PFOR) {
              var vertexBuffer =
                  DecodingUtils.decodeFastPforDeltaCoordinates(
                      tile,
                      geometryStreamMetadata.numValues(),
                      geometryStreamMetadata.byteLength(),
                      offset);
              vertexList = Arrays.stream(vertexBuffer).boxed().collect(Collectors.toList());
            } else {
              vertexList =
                  IntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, true);
            }
          } else {
            vertexList =
                IntegerDecoder.decodeMortonStream(
                    tile, offset, (MortonEncodedStreamMetadata) geometryStreamMetadata);
          }
          break;
      }
    }

    return new GeometryColumn(
        geometryTypes, numGeometries, numParts, numRings, vertexOffsets, vertexList);
  }

  public static Geometry[] decodeGeometry(GeometryColumn geometryColumn) {
    var geometries = new Geometry[geometryColumn.geometryTypes.size()];
    var partOffsetCounter = 0;
    var ringOffsetsCounter = 0;
    var geometryOffsetsCounter = 0;
    var geometryCounter = 0;
    var geometryFactory = new GeometryFactory();
    var vertexBufferOffset = 0;
    var vertexOffsetsOffset = 0;

    var geometryTypes = geometryColumn.geometryTypes();
    var geometryOffsets = geometryColumn.numGeometries();
    var partOffsets = geometryColumn.numParts();
    var ringOffsets = geometryColumn.numRings();
    var vertexOffsets =
        geometryColumn.vertexOffsets() != null
            ? geometryColumn.vertexOffsets().stream().mapToInt(i -> i).toArray()
            : null;

    var vertexBuffer = geometryColumn.vertexList.stream().mapToInt(i -> i).toArray();

    var containsPolygon =
        geometryColumn.geometryTypes.stream()
            .anyMatch(
                g ->
                    g == GeometryType.POLYGON.ordinal()
                        || g == GeometryType.MULTIPOLYGON.ordinal());
    // TODO: refactor redundant code
    for (var geometryType : geometryTypes) {
      if (geometryType.equals(GeometryType.POINT.ordinal())) {
        if (vertexOffsets == null || vertexOffsets.length == 0) {
          var x = vertexBuffer[vertexBufferOffset++];
          var y = vertexBuffer[vertexBufferOffset++];
          var coordinate = new Coordinate(x, y);
          geometries[geometryCounter++] = geometryFactory.createPoint(coordinate);
        } else {
          var offset = vertexOffsets[vertexOffsetsOffset++] * 2;
          var x = vertexBuffer[offset];
          var y = vertexBuffer[offset + 1];
          var coordinate = new Coordinate(x, y);
          geometries[geometryCounter++] = geometryFactory.createPoint(coordinate);
        }
      } else if (geometryType.equals(GeometryType.MULTIPOINT.ordinal())) {
        var numPoints = geometryOffsets.get(geometryOffsetsCounter++);
        var points = new Point[numPoints];
        if (vertexOffsets == null || vertexOffsets.length == 0) {
          for (var i = 0; i < numPoints; i++) {
            var x = vertexBuffer[vertexBufferOffset++];
            var y = vertexBuffer[vertexBufferOffset++];
            var coordinate = new Coordinate(x, y);
            points[i] = geometryFactory.createPoint(coordinate);
          }
          geometries[geometryCounter++] = geometryFactory.createMultiPoint(points);
        } else {
          for (var i = 0; i < numPoints; i++) {
            var offset = vertexOffsets[vertexOffsetsOffset++] * 2;
            var x = vertexBuffer[offset];
            var y = vertexBuffer[offset + 1];
            var coordinate = new Coordinate(x, y);
            points[i] = geometryFactory.createPoint(coordinate);
          }
          geometries[geometryCounter++] = geometryFactory.createMultiPoint(points);
        }
      } else if (geometryType.equals(GeometryType.LINESTRING.ordinal())) {
        var numVertices =
            containsPolygon
                ? ringOffsets.get(ringOffsetsCounter++)
                : partOffsets.get(partOffsetCounter++);

        if (vertexOffsets == null || vertexOffsets.length == 0) {
          var vertices = getLineString(vertexBuffer, vertexBufferOffset, numVertices, false);
          vertexBufferOffset += numVertices * 2;
          geometries[geometryCounter++] = geometryFactory.createLineString(vertices);
        } else {
          var vertices =
              decodeDictionaryEncodedLineString(
                  vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, false);
          vertexOffsetsOffset += numVertices;

          geometries[geometryCounter++] = geometryFactory.createLineString(vertices);
        }
      } else if (geometryType.equals(GeometryType.POLYGON.ordinal())) {
        var numRings = partOffsets.get(partOffsetCounter++);
        var rings = new LinearRing[numRings - 1];
        var numVertices = ringOffsets.get(ringOffsetsCounter++);
        if (vertexOffsets == null || vertexOffsets.length == 0) {
          LinearRing shell =
              getLinearRing(vertexBuffer, vertexBufferOffset, numVertices, geometryFactory);
          vertexBufferOffset += numVertices * 2;
          for (var i = 0; i < rings.length; i++) {
            numVertices = ringOffsets.get(ringOffsetsCounter++);
            rings[i] =
                getLinearRing(vertexBuffer, vertexBufferOffset, numVertices, geometryFactory);
            vertexBufferOffset += numVertices * 2;
          }
          geometries[geometryCounter++] = geometryFactory.createPolygon(shell, rings);
        } else {
          LinearRing shell =
              decodeDictionaryEncodedLinearRing(
                  vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, geometryFactory);
          vertexOffsetsOffset += numVertices;
          for (var i = 0; i < rings.length; i++) {
            numVertices = ringOffsets.get(ringOffsetsCounter++);
            rings[i] =
                decodeDictionaryEncodedLinearRing(
                    vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, geometryFactory);
            vertexOffsetsOffset += numVertices;
          }
          geometries[geometryCounter++] = geometryFactory.createPolygon(shell, rings);
        }
      } else if (geometryType.equals(GeometryType.MULTILINESTRING.ordinal())) {
        var numLineStrings = geometryOffsets.get(geometryOffsetsCounter++);
        var lineStrings = new LineString[numLineStrings];
        if (vertexOffsets == null || vertexOffsets.length == 0) {
          for (var i = 0; i < numLineStrings; i++) {
            var numVertices =
                containsPolygon
                    ? ringOffsets.get(ringOffsetsCounter++)
                    : partOffsets.get(partOffsetCounter++);

            var vertices = getLineString(vertexBuffer, vertexBufferOffset, numVertices, false);
            lineStrings[i] = geometryFactory.createLineString(vertices);
            vertexBufferOffset += numVertices * 2;
          }
          geometries[geometryCounter++] = geometryFactory.createMultiLineString(lineStrings);
        } else {
          for (var i = 0; i < numLineStrings; i++) {
            var numVertices =
                containsPolygon
                    ? ringOffsets.get(ringOffsetsCounter++)
                    : partOffsets.get(partOffsetCounter++);

            var vertices =
                decodeDictionaryEncodedLineString(
                    vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, false);
            lineStrings[i] = geometryFactory.createLineString(vertices);
            vertexOffsetsOffset += numVertices;
          }
          geometries[geometryCounter++] = geometryFactory.createMultiLineString(lineStrings);
        }
      } else if (geometryType.equals(GeometryType.MULTIPOLYGON.ordinal())) {
        var numPolygons = geometryOffsets.get(geometryOffsetsCounter++);
        var polygons = new Polygon[numPolygons];
        var numVertices = 0;
        if (vertexOffsets == null || vertexOffsets.length == 0) {
          for (var i = 0; i < numPolygons; i++) {
            var numRings = partOffsets.get(partOffsetCounter++);
            var rings = new LinearRing[numRings - 1];
            numVertices = ringOffsets.get(ringOffsetsCounter++);
            LinearRing shell =
                getLinearRing(vertexBuffer, vertexBufferOffset, numVertices, geometryFactory);
            vertexBufferOffset += numVertices * 2;
            for (var j = 0; j < rings.length; j++) {
              var numRingVertices = ringOffsets.get(ringOffsetsCounter++);
              rings[j] =
                  getLinearRing(vertexBuffer, vertexBufferOffset, numRingVertices, geometryFactory);
              vertexBufferOffset += numRingVertices * 2;
            }

            polygons[i] = geometryFactory.createPolygon(shell, rings);
          }
          geometries[geometryCounter++] = geometryFactory.createMultiPolygon(polygons);
        } else {
          for (var i = 0; i < numPolygons; i++) {
            var numRings = partOffsets.get(partOffsetCounter++);
            var rings = new LinearRing[numRings - 1];
            numVertices = ringOffsets.get(ringOffsetsCounter++);
            LinearRing shell =
                decodeDictionaryEncodedLinearRing(
                    vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, geometryFactory);
            vertexOffsetsOffset += numVertices;
            for (var j = 0; j < rings.length; j++) {
              numVertices = ringOffsets.get(ringOffsetsCounter++);
              rings[j] =
                  decodeDictionaryEncodedLinearRing(
                      vertexBuffer,
                      vertexOffsets,
                      vertexOffsetsOffset,
                      numVertices,
                      geometryFactory);
              vertexOffsetsOffset += numVertices;
            }
            polygons[i] = geometryFactory.createPolygon(shell, rings);
          }
          geometries[geometryCounter++] = geometryFactory.createMultiPolygon(polygons);
        }
      } else {
        throw new IllegalArgumentException(
            "The specified geometry type is currently not supported: " + geometryType);
      }
    }

    return geometries;
  }

  public static Geometry[] decodeGeometryVectorized(GeometryVector geometryVector) {
    var geometries = new Geometry[geometryVector.numGeometries];
    var partOffsetCounter = 1;
    var ringOffsetsCounter = 1;
    var geometryOffsetsCounter = 1;
    var geometryCounter = 0;
    var geometryFactory = new GeometryFactory();
    var vertexBufferOffset = 0;
    var vertexOffsetsOffset = 0;

    GeometryVector.MortonSettings mortonSettings = geometryVector.mortonSettings.orElse(null);
    var topologyVector = geometryVector.topologyVector;
    var geometryOffsets = topologyVector.geometryOffsets();
    var partOffsets = topologyVector.partOffsets();
    var ringOffsets = topologyVector.ringOffsets();
    var vertexOffsets =
        geometryVector.vertexOffsets != null ? geometryVector.vertexOffsets.array() : null;

    // TODO: get rid of that extra step
    var containsPolygon = geometryVector.containsPolygonGeometry();
    var vertexBuffer = geometryVector.vertexBuffer.array();
    // TODO: refactor redundant code
    for (var i = 0; i < geometryVector.numGeometries; i++) {
      var geometryType = geometryVector.getGeometryType(i);
      if (geometryType == GeometryType.POINT.ordinal()) {
        if (vertexOffsets == null || vertexOffsets.length == 0) {
          var x = vertexBuffer[vertexBufferOffset++];
          var y = vertexBuffer[vertexBufferOffset++];
          var coordinate = new Coordinate(x, y);
          geometries[geometryCounter++] = geometryFactory.createPoint(coordinate);
        } else if (geometryVector.vertexBufferType == GeometryVector.VertexBufferType.VEC_2) {
          var offset = vertexOffsets[vertexOffsetsOffset++] * 2;
          var x = vertexBuffer[offset];
          var y = vertexBuffer[offset + 1];
          var coordinate = new Coordinate(x, y);
          geometries[geometryCounter++] = geometryFactory.createPoint(coordinate);
        } else {

          var offset = vertexOffsets[vertexOffsetsOffset++];
          var mortonCode = vertexBuffer[offset];
          var vertex =
              ZOrderCurve.decode(
                  mortonCode, mortonSettings.numBits, mortonSettings.coordinateShift);
          var coordinate = new Coordinate(vertex[0], vertex[1]);
          geometries[geometryCounter++] = geometryFactory.createPoint(coordinate);
        }

        if (geometryOffsets != null) {
          geometryOffsetsCounter++;
        }
        if (partOffsets != null) {
          partOffsetCounter++;
        }
        if (ringOffsets != null) {
          ringOffsetsCounter++;
        }
      } else if (geometryType == GeometryType.MULTIPOINT.ordinal()) {
        var numPoints =
            geometryOffsets.get(geometryOffsetsCounter)
                - geometryOffsets.get(geometryOffsetsCounter - 1);
        geometryOffsetsCounter++;
        var points = new Point[numPoints];
        if (vertexOffsets == null || vertexOffsets.length == 0) {
          for (var j = 0; j < numPoints; j++) {
            var x = vertexBuffer[vertexBufferOffset++];
            var y = vertexBuffer[vertexBufferOffset++];
            var coordinate = new Coordinate(x, y);
            points[j] = geometryFactory.createPoint(coordinate);
          }
          geometries[geometryCounter++] = geometryFactory.createMultiPoint(points);
        } else {
          for (var j = 0; j < numPoints; j++) {
            var offset = vertexOffsets[vertexOffsetsOffset++] * 2;
            var x = vertexBuffer[offset];
            var y = vertexBuffer[offset + 1];
            var coordinate = new Coordinate(x, y);
            points[j] = geometryFactory.createPoint(coordinate);
          }
          geometries[geometryCounter++] = geometryFactory.createMultiPoint(points);
        }
      } else if (geometryType == GeometryType.LINESTRING.ordinal()) {
        var numVertices = 0;
        if (containsPolygon) {
          numVertices =
              ringOffsets.get(ringOffsetsCounter) - ringOffsets.get(ringOffsetsCounter - 1);
          ringOffsetsCounter++;
        } else {
          numVertices = partOffsets.get(partOffsetCounter) - partOffsets.get(partOffsetCounter - 1);
        }
        partOffsetCounter++;

        Coordinate[] vertices;
        if (vertexOffsets == null || vertexOffsets.length == 0) {
          vertices = getLineString(vertexBuffer, vertexBufferOffset, numVertices, false);
          vertexBufferOffset += numVertices * 2;
        } else {
          /* Currently only 2D coordinates are supported in this implementation */
          vertices =
              geometryVector.vertexBufferType == GeometryVector.VertexBufferType.VEC_2
                  ? decodeDictionaryEncodedLineString(
                      vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, false)
                  : decodeMortonDictionaryEncodedLineString(
                      vertexBuffer,
                      vertexOffsets,
                      vertexOffsetsOffset,
                      numVertices,
                      false,
                      mortonSettings);
          vertexOffsetsOffset += numVertices;
        }

        geometries[geometryCounter++] = geometryFactory.createLineString(vertices);

        if (geometryOffsets != null) {
          geometryOffsetsCounter++;
        }
      } else if (geometryType == GeometryType.POLYGON.ordinal()) {
        var numRings = partOffsets.get(partOffsetCounter) - partOffsets.get(partOffsetCounter - 1);
        partOffsetCounter++;
        var rings = new LinearRing[numRings - 1];
        var numVertices =
            ringOffsets.get(ringOffsetsCounter) - ringOffsets.get(ringOffsetsCounter - 1);
        ringOffsetsCounter++;

        if (vertexOffsets == null || vertexOffsets.length == 0) {
          LinearRing shell =
              getLinearRing(vertexBuffer, vertexBufferOffset, numVertices, geometryFactory);
          vertexBufferOffset += numVertices * 2;
          for (var j = 0; j < rings.length; j++) {
            numVertices =
                ringOffsets.get(ringOffsetsCounter) - ringOffsets.get(ringOffsetsCounter - 1);
            ringOffsetsCounter++;
            rings[j] =
                getLinearRing(vertexBuffer, vertexBufferOffset, numVertices, geometryFactory);
            vertexBufferOffset += numVertices * 2;
          }
          geometries[geometryCounter++] = geometryFactory.createPolygon(shell, rings);
        } else {
          LinearRing shell =
              geometryVector.vertexBufferType == GeometryVector.VertexBufferType.VEC_2
                  ? decodeDictionaryEncodedLinearRing(
                      vertexBuffer,
                      vertexOffsets,
                      vertexOffsetsOffset,
                      numVertices,
                      geometryFactory)
                  : decodeMortonDictionaryEncodedLinearRing(
                      vertexBuffer,
                      vertexOffsets,
                      vertexOffsetsOffset,
                      numVertices,
                      geometryFactory,
                      mortonSettings);
          vertexOffsetsOffset += numVertices;
          for (var j = 0; j < rings.length; j++) {
            numVertices =
                ringOffsets.get(ringOffsetsCounter) - ringOffsets.get(ringOffsetsCounter - 1);
            ringOffsetsCounter++;
            rings[j] =
                geometryVector.vertexBufferType == GeometryVector.VertexBufferType.VEC_2
                    ? decodeDictionaryEncodedLinearRing(
                        vertexBuffer,
                        vertexOffsets,
                        vertexOffsetsOffset,
                        numVertices,
                        geometryFactory)
                    : decodeMortonDictionaryEncodedLinearRing(
                        vertexBuffer,
                        vertexOffsets,
                        vertexOffsetsOffset,
                        numVertices,
                        geometryFactory,
                        mortonSettings);

            vertexOffsetsOffset += numVertices;
          }
          geometries[geometryCounter++] = geometryFactory.createPolygon(shell, rings);
        }

        if (geometryOffsets != null) {
          geometryOffsetsCounter++;
        }
      } else if (geometryType == GeometryType.MULTILINESTRING.ordinal()) {
        var numLineStrings =
            geometryOffsets.get(geometryOffsetsCounter)
                - geometryOffsets.get(geometryOffsetsCounter - 1);
        geometryOffsetsCounter++;
        var lineStrings = new LineString[numLineStrings];
        if (vertexOffsets == null || vertexOffsets.length == 0) {
          for (var j = 0; j < numLineStrings; j++) {
            var numVertices = 0;
            if (containsPolygon) {
              numVertices =
                  ringOffsets.get(ringOffsetsCounter) - ringOffsets.get(ringOffsetsCounter - 1);
              ringOffsetsCounter++;
            } else {
              numVertices =
                  partOffsets.get(partOffsetCounter) - partOffsets.get(partOffsetCounter - 1);
            }
            partOffsetCounter++;

            var vertices = getLineString(vertexBuffer, vertexBufferOffset, numVertices, false);
            lineStrings[j] = geometryFactory.createLineString(vertices);
            vertexBufferOffset += numVertices * 2;
          }
          geometries[geometryCounter++] = geometryFactory.createMultiLineString(lineStrings);
        } else {
          for (var j = 0; j < numLineStrings; j++) {
            var numVertices = 0;
            if (containsPolygon) {
              numVertices =
                  ringOffsets.get(ringOffsetsCounter) - ringOffsets.get(ringOffsetsCounter - 1);
              ringOffsetsCounter++;
            } else {
              numVertices =
                  partOffsets.get(partOffsetCounter) - partOffsets.get(partOffsetCounter - 1);
            }
            partOffsetCounter++;

            var vertices =
                geometryVector.vertexBufferType == GeometryVector.VertexBufferType.VEC_2
                    ? decodeDictionaryEncodedLineString(
                        vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, false)
                    : decodeMortonDictionaryEncodedLineString(
                        vertexBuffer,
                        vertexOffsets,
                        vertexOffsetsOffset,
                        numVertices,
                        false,
                        mortonSettings);
            lineStrings[j] = geometryFactory.createLineString(vertices);
            vertexOffsetsOffset += numVertices;
          }
          geometries[geometryCounter++] = geometryFactory.createMultiLineString(lineStrings);
        }
      } else if (geometryType == GeometryType.MULTIPOLYGON.ordinal()) {
        var numPolygons =
            geometryOffsets.get(geometryOffsetsCounter)
                - geometryOffsets.get(geometryOffsetsCounter - 1);
        geometryOffsetsCounter++;
        var polygons = new Polygon[numPolygons];
        var numVertices = 0;
        if (vertexOffsets == null || vertexOffsets.length == 0) {
          for (var j = 0; j < numPolygons; j++) {
            var numRings =
                partOffsets.get(partOffsetCounter) - partOffsets.get(partOffsetCounter - 1);
            partOffsetCounter++;
            var rings = new LinearRing[numRings - 1];
            numVertices =
                ringOffsets.get(ringOffsetsCounter) - ringOffsets.get(ringOffsetsCounter - 1);
            ringOffsetsCounter++;
            LinearRing shell =
                getLinearRing(vertexBuffer, vertexBufferOffset, numVertices, geometryFactory);
            vertexBufferOffset += numVertices * 2;
            for (var k = 0; k < rings.length; k++) {
              var numRingVertices =
                  ringOffsets.get(ringOffsetsCounter) - ringOffsets.get(ringOffsetsCounter - 1);
              ringOffsetsCounter++;
              rings[k] =
                  getLinearRing(vertexBuffer, vertexBufferOffset, numRingVertices, geometryFactory);
              vertexBufferOffset += numRingVertices * 2;
            }

            polygons[j] = geometryFactory.createPolygon(shell, rings);
          }
          geometries[geometryCounter++] = geometryFactory.createMultiPolygon(polygons);
        } else {
          for (var j = 0; j < numPolygons; j++) {
            var numRings =
                partOffsets.get(partOffsetCounter) - partOffsets.get(partOffsetCounter - 1);
            partOffsetCounter++;
            var rings = new LinearRing[numRings - 1];
            numVertices =
                ringOffsets.get(ringOffsetsCounter) - ringOffsets.get(ringOffsetsCounter - 1);
            ringOffsetsCounter++;
            var shell =
                geometryVector.vertexBufferType == GeometryVector.VertexBufferType.VEC_2
                    ? decodeDictionaryEncodedLinearRing(
                        vertexBuffer,
                        vertexOffsets,
                        vertexOffsetsOffset,
                        numVertices,
                        geometryFactory)
                    : decodeMortonDictionaryEncodedLinearRing(
                        vertexBuffer,
                        vertexOffsets,
                        vertexOffsetsOffset,
                        numVertices,
                        geometryFactory,
                        mortonSettings);
            vertexOffsetsOffset += numVertices;
            for (var k = 0; k < rings.length; k++) {
              numVertices =
                  (ringOffsets.get(ringOffsetsCounter) - ringOffsets.get(ringOffsetsCounter - 1));
              ringOffsetsCounter++;
              rings[k] =
                  geometryVector.vertexBufferType == GeometryVector.VertexBufferType.VEC_2
                      ? decodeDictionaryEncodedLinearRing(
                          vertexBuffer,
                          vertexOffsets,
                          vertexOffsetsOffset,
                          numVertices,
                          geometryFactory)
                      : decodeMortonDictionaryEncodedLinearRing(
                          vertexBuffer,
                          vertexOffsets,
                          vertexOffsetsOffset,
                          numVertices,
                          geometryFactory,
                          mortonSettings);
              vertexOffsetsOffset += numVertices;
            }

            polygons[j] = geometryFactory.createPolygon(shell, rings);
          }
          geometries[geometryCounter++] = geometryFactory.createMultiPolygon(polygons);
        }
      } else {
        throw new IllegalArgumentException(
            "The specified geometry type is currently not supported.");
      }
    }

    return geometries;
  }

  private static LinearRing getLinearRing(
      int[] vertexBuffer, int startIndex, int numVertices, GeometryFactory geometryFactory) {
    var linearRing = getLineString(vertexBuffer, startIndex, numVertices, true);
    return geometryFactory.createLinearRing(linearRing);
  }

  private static LinearRing decodeDictionaryEncodedLinearRing(
      int[] vertexBuffer,
      int[] vertexOffsets,
      int vertexOffset,
      int numVertices,
      GeometryFactory geometryFactory) {
    var linearRing =
        decodeDictionaryEncodedLineString(
            vertexBuffer, vertexOffsets, vertexOffset, numVertices, true);
    return geometryFactory.createLinearRing(linearRing);
  }

  private static LinearRing decodeMortonDictionaryEncodedLinearRing(
      int[] vertexBuffer,
      int[] vertexOffsets,
      int vertexOffset,
      int numVertices,
      GeometryFactory geometryFactory,
      GeometryVector.MortonSettings mortonSettings) {
    var linearRing =
        decodeMortonDictionaryEncodedLineString(
            vertexBuffer, vertexOffsets, vertexOffset, numVertices, true, mortonSettings);
    return geometryFactory.createLinearRing(linearRing);
  }

  private static Coordinate[] getLineString(
      int[] vertexBuffer, int startIndex, int numVertices, boolean closeLineString) {
    var vertices = new Coordinate[closeLineString ? numVertices + 1 : numVertices];
    for (var i = 0; i < numVertices * 2; i += 2) {
      var x = vertexBuffer[startIndex + i];
      var y = vertexBuffer[startIndex + i + 1];
      vertices[i / 2] = new Coordinate(x, y);
    }

    if (closeLineString) {
      vertices[vertices.length - 1] = vertices[0];
    }
    return vertices;
  }

  private static Coordinate[] decodeDictionaryEncodedLineString(
      int[] vertexBuffer,
      int[] vertexOffsets,
      int vertexOffset,
      int numVertices,
      boolean closeLineString) {
    var vertices = new Coordinate[closeLineString ? numVertices + 1 : numVertices];
    for (var i = 0; i < numVertices * 2; i += 2) {
      var offset = vertexOffsets[vertexOffset + i / 2] * 2;
      var x = vertexBuffer[offset];
      var y = vertexBuffer[offset + 1];
      vertices[i / 2] = new Coordinate(x, y);
    }

    if (closeLineString) {
      vertices[vertices.length - 1] = vertices[0];
    }
    return vertices;
  }

  /*
   * The decoding of the Morton encoded vertices can happen completely in parallel on the GPU in the Vertex or Compute Shader.
   * Therefore, the decoding of the Morton encoded vertices is not part of the decoding benchmark from the storage into the
   * in-memory representation.
   * */
  private static Coordinate[] decodeMortonDictionaryEncodedLineString(
      int[] vertexBuffer,
      int[] vertexOffsets,
      int vertexOffset,
      int numVertices,
      boolean closeLineString,
      GeometryVector.MortonSettings mortonSettings) {
    var vertices = new Coordinate[closeLineString ? numVertices + 1 : numVertices];
    for (var i = 0; i < numVertices; i++) {
      var offset = vertexOffsets[vertexOffset + i];
      var mortonEncodedVertex = vertexBuffer[offset];
      // TODO: refactor to use instance methods
      var vertex =
          ZOrderCurve.decode(
              mortonEncodedVertex, mortonSettings.numBits, mortonSettings.coordinateShift);
      vertices[i] = new Coordinate(vertex[0], vertex[1]);
    }
    if (closeLineString) {
      vertices[vertices.length - 1] = vertices[0];
    }

    return vertices;
  }
}
