package com.mlt.converter.encodings;

import static com.mlt.converter.encodings.IntegerEncoder.encodeFastPfor;
import static com.mlt.converter.encodings.IntegerEncoder.encodeVarint;

import com.mlt.converter.CollectionUtils;
import com.mlt.converter.geometry.*;
import com.mlt.metadata.stream.*;
import java.util.*;
import java.util.function.Function;
import java.util.stream.Collectors;
import java.util.stream.IntStream;
import org.apache.commons.lang3.ArrayUtils;
import org.locationtech.jts.geom.Geometry;
import org.locationtech.jts.geom.GeometryFactory;
import org.locationtech.jts.geom.LineString;
import org.locationtech.jts.geom.MultiLineString;
import org.locationtech.jts.geom.MultiPoint;
import org.locationtech.jts.geom.MultiPolygon;
import org.locationtech.jts.geom.Point;
import org.locationtech.jts.geom.Polygon;

public class GeometryEncoder {

  public record EncodedGeometryColumn(
      int numStreams, byte[] encodedValues, int maxVertexValue, boolean geometryColumnSorted) {}

  public record SortSettings(boolean isSortable, List<Long> featureIds) {}

  private GeometryEncoder() {}

  // TODO: add selection algorithms based on statistics and sampling
  public static EncodedGeometryColumn encodeGeometryColumn(
      List<Geometry> geometries,
      PhysicalLevelTechnique physicalLevelTechnique,
      SortSettings sortSettings) {
    var geometryTypes = new ArrayList<Integer>();
    var numGeometries = new ArrayList<Integer>();
    var numParts = new ArrayList<Integer>();
    var numRings = new ArrayList<Integer>();
    var vertexBuffer = new ArrayList<Vertex>();
    var containsPolygon =
        geometries.stream()
            .anyMatch(
                g ->
                    g.getGeometryType().equals(Geometry.TYPENAME_MULTIPOLYGON)
                        || g.getGeometryType().equals(Geometry.TYPENAME_POLYGON));
    for (var geometry : geometries) {
      var geometryType = geometry.getGeometryType();
      switch (geometryType) {
        case Geometry.TYPENAME_POINT:
          {
            geometryTypes.add(GeometryType.POINT.ordinal());
            var point = (Point) geometry;
            var x = (int) point.getX();
            var y = (int) point.getY();
            vertexBuffer.add(new Vertex(x, y));
            break;
          }
        case Geometry.TYPENAME_LINESTRING:
          {
            // TODO: verify if part of a MultiPolygon or Polygon geometry add then to numRings?
            geometryTypes.add(GeometryType.LINESTRING.ordinal());
            var lineString = (LineString) geometry;
            var numVertices = lineString.getCoordinates().length;
            addLineString(containsPolygon, numVertices, numParts, numRings);
            var vertices = flatLineString(lineString);
            vertexBuffer.addAll(vertices);
            break;
          }
        case Geometry.TYPENAME_POLYGON:
          {
            geometryTypes.add(GeometryType.POLYGON.ordinal());
            var polygon = (Polygon) geometry;
            var vertices = flatPolygon(polygon, numParts, numRings);
            vertexBuffer.addAll(vertices);
            break;
          }
        case Geometry.TYPENAME_MULTILINESTRING:
          {
            // TODO: verify if part of a MultiPolygon or Polygon geometry add then to numRings?
            geometryTypes.add(GeometryType.MULTILINESTRING.ordinal());
            var multiLineString = (MultiLineString) geometry;
            var numLineStrings = multiLineString.getNumGeometries();
            numGeometries.add(numLineStrings);
            for (var i = 0; i < numLineStrings; i++) {
              var lineString = (LineString) multiLineString.getGeometryN(i);
              var numVertices = lineString.getCoordinates().length;
              addLineString(containsPolygon, numVertices, numParts, numRings);
              vertexBuffer.addAll(flatLineString(lineString));
            }
            break;
          }
        case Geometry.TYPENAME_MULTIPOLYGON:
          {
            geometryTypes.add(GeometryType.MULTIPOLYGON.ordinal());
            var multiPolygon = (MultiPolygon) geometry;
            var numPolygons = multiPolygon.getNumGeometries();
            numGeometries.add(numPolygons);
            for (var i = 0; i < numPolygons; i++) {
              var polygon = (Polygon) multiPolygon.getGeometryN(i);
              var vertices = flatPolygon(polygon, numParts, numRings);
              vertexBuffer.addAll(vertices);
            }
            break;
          }
        case Geometry.TYPENAME_MULTIPOINT:
          {
            geometryTypes.add(GeometryType.MULTIPOINT.ordinal());
            var multiPoint = (MultiPoint) geometry;
            var numPoints = multiPoint.getNumGeometries();
            numGeometries.add(numPoints);
            for (var i = 0; i < numPoints; i++) {
              var point = (Point) multiPoint.getGeometryN(i);
              var x = (int) point.getX();
              var y = (int) point.getY();
              vertexBuffer.add(new Vertex(x, y));
            }
            break;
          }
        default:
          throw new IllegalArgumentException(
              "Specified geometry type is not (yet) supported: " + geometryType);
      }
    }

    // TODO: get rid of that separate calculation
    var minVertexValue =
        Collections.min(
            vertexBuffer.stream().flatMapToInt(v -> IntStream.of(v.x(), v.y())).boxed().toList());
    var maxVertexValue =
        Collections.max(
            vertexBuffer.stream().flatMapToInt(v -> IntStream.of(v.x(), v.y())).boxed().toList());

    var hilbertCurve = new HilbertCurve(minVertexValue, maxVertexValue);
    var zOrderCurve = new ZOrderCurve(minVertexValue, maxVertexValue);
    // TODO: if the ratio is lower then 2 dictionary encoding has not to be considered?
    var vertexDictionary = addVerticesToDictionary(vertexBuffer, hilbertCurve);
    var mortonEncodedDictionary = addVerticesToMortonDictionary(vertexBuffer, zOrderCurve);

    int[] hilbertIds = vertexDictionary.keySet().stream().mapToInt(d -> d).toArray();
    var dictionaryOffsets =
        getVertexOffsets(vertexBuffer, (id) -> Arrays.binarySearch(hilbertIds, id), hilbertCurve);

    int[] mortonIds = mortonEncodedDictionary.stream().mapToInt(d -> d).toArray();
    var mortonEncodedDictionaryOffsets =
        getVertexOffsets(vertexBuffer, (id) -> Arrays.binarySearch(mortonIds, id), zOrderCurve);

    /* Test if Plain, Vertex Dictionary or Morton Encoded Vertex Dictionary is the most efficient
     * -> Plain -> convert VertexBuffer with Delta Encoding and specified Physical Level Technique
     * -> Dictionary -> convert VertexOffsets with IntegerEncoder and VertexBuffer with Delta Encoding and specified Physical Level Technique
     * -> Morton Encoded Dictionary -> convert VertexOffsets with Integer Encoder and VertexBuffer with IntegerEncoder
     * */
    var zigZagDeltaVertexBuffer = zigZagDeltaEncodeVertices(vertexBuffer);
    var zigZagDeltaVertexDictionary = zigZagDeltaEncodeVertices(vertexDictionary.values());

    // TODO: get rid of that conversions
    // TODO: should we do a potential recursive encoding again
    var encodedVertexBuffer =
        IntegerEncoder.encodeInt(
            Arrays.stream(zigZagDeltaVertexBuffer).boxed().collect(Collectors.toList()),
            physicalLevelTechnique,
            false);
    // TODO: should we do a potential recursive encoding again
    var encodedVertexDictionary =
        IntegerEncoder.encodeInt(
            Arrays.stream(zigZagDeltaVertexDictionary).boxed().collect(Collectors.toList()),
            physicalLevelTechnique,
            false);
    var encodedMortonVertexDictionary =
        IntegerEncoder.encodeMortonCodes(
            new ArrayList<>(mortonEncodedDictionary), physicalLevelTechnique);
    var encodedDictionaryOffsets =
        IntegerEncoder.encodeInt(dictionaryOffsets, physicalLevelTechnique, false);
    var encodedMortonEncodedDictionaryOffsets =
        IntegerEncoder.encodeInt(mortonEncodedDictionaryOffsets, physicalLevelTechnique, false);

    // TODO: refactor this simple approach to also work with mixed geometries
    var geometryColumnSorted = false;
    if (sortSettings.isSortable && numGeometries.isEmpty() && numRings.isEmpty()) {
      if (numParts.size() == sortSettings.featureIds.size()) {
        /* Currently the VertexOffsets are only sorted if all geometries in the geometry column are of type LineString */
        GeometryUtils.sortVertexOffsets(
            numParts, mortonEncodedDictionaryOffsets, sortSettings.featureIds());
        encodedMortonEncodedDictionaryOffsets =
            IntegerEncoder.encodeInt(mortonEncodedDictionaryOffsets, physicalLevelTechnique, false);
        geometryColumnSorted = true;
      } else if (numParts.isEmpty()) {
        GeometryUtils.sortPoints(vertexBuffer, hilbertCurve, sortSettings.featureIds);
        zigZagDeltaVertexBuffer = zigZagDeltaEncodeVertices(vertexBuffer);
        encodedVertexBuffer =
            IntegerEncoder.encodeInt(
                Arrays.stream(zigZagDeltaVertexBuffer).boxed().collect(Collectors.toList()),
                physicalLevelTechnique,
                false);
        geometryColumnSorted = true;
      }
    }

    var encodedTopologyStreams =
        IntegerEncoder.encodeIntStream(
            geometryTypes, physicalLevelTechnique, false, PhysicalStreamType.LENGTH, null);
    var numStreams = 1;
    if (!numGeometries.isEmpty()) {
      var encodedNumGeometries =
          IntegerEncoder.encodeIntStream(
              numGeometries,
              physicalLevelTechnique,
              false,
              PhysicalStreamType.LENGTH,
              new LogicalStreamType(LengthType.GEOMETRIES));
      encodedTopologyStreams = ArrayUtils.addAll(encodedTopologyStreams, encodedNumGeometries);
      numStreams++;
    }
    if (!numParts.isEmpty()) {
      var encodedNumParts =
          IntegerEncoder.encodeIntStream(
              numParts,
              physicalLevelTechnique,
              false,
              PhysicalStreamType.LENGTH,
              new LogicalStreamType(LengthType.PARTS));
      encodedTopologyStreams = ArrayUtils.addAll(encodedTopologyStreams, encodedNumParts);
      numStreams++;
    }
    if (!numRings.isEmpty()) {
      var encodedNumRings =
          IntegerEncoder.encodeIntStream(
              numRings,
              physicalLevelTechnique,
              false,
              PhysicalStreamType.LENGTH,
              new LogicalStreamType(LengthType.RINGS));
      encodedTopologyStreams = ArrayUtils.addAll(encodedTopologyStreams, encodedNumRings);
      numStreams++;
    }

    var plainVertexBufferSize = encodedVertexBuffer.encodedValues.length;
    var dictionaryEncodedSize =
        encodedDictionaryOffsets.encodedValues.length
            + encodedVertexDictionary.encodedValues.length;
    var mortonDictionaryEncodedSize =
        encodedMortonEncodedDictionaryOffsets.encodedValues.length
            + encodedMortonVertexDictionary.encodedValues.length;

    if (plainVertexBufferSize <= dictionaryEncodedSize
        && plainVertexBufferSize <= mortonDictionaryEncodedSize) {
      // TODO: get rid of extra conversion
      var encodedVertexBufferStream =
          encodeVertexBuffer(
              Arrays.stream(zigZagDeltaVertexBuffer).boxed().collect(Collectors.toList()),
              physicalLevelTechnique);

      return new EncodedGeometryColumn(
          numStreams + 1,
          ArrayUtils.addAll(encodedTopologyStreams, encodedVertexBufferStream),
          maxVertexValue,
          geometryColumnSorted);
    } else if (dictionaryEncodedSize < plainVertexBufferSize
        && dictionaryEncodedSize <= mortonDictionaryEncodedSize) {
      var encodedVertexOffsetStream =
          IntegerEncoder.encodeIntStream(
              dictionaryOffsets,
              physicalLevelTechnique,
              false,
              PhysicalStreamType.OFFSET,
              new LogicalStreamType(OffsetType.VERTEX));
      var encodedVertexDictionaryStream =
          encodeVertexBuffer(
              Arrays.stream(zigZagDeltaVertexDictionary).boxed().collect(Collectors.toList()),
              physicalLevelTechnique);

      return new EncodedGeometryColumn(
          numStreams + 2,
          CollectionUtils.concatByteArrays(
              encodedTopologyStreams, encodedVertexOffsetStream, encodedVertexDictionaryStream),
          maxVertexValue,
          false);
    } else {
      var encodedMortonVertexOffsetStream =
          IntegerEncoder.encodeIntStream(
              mortonEncodedDictionaryOffsets,
              physicalLevelTechnique,
              false,
              PhysicalStreamType.OFFSET,
              new LogicalStreamType(OffsetType.VERTEX));

      var encodedMortonEncodedVertexDictionaryStream =
          IntegerEncoder.encodeMortonStream(
              new ArrayList<>(mortonEncodedDictionary),
              zOrderCurve.numBits(),
              zOrderCurve.coordinateShift(),
              physicalLevelTechnique);

      return new EncodedGeometryColumn(
          numStreams + 2,
          CollectionUtils.concatByteArrays(
              encodedTopologyStreams,
              encodedMortonVertexOffsetStream,
              encodedMortonEncodedVertexDictionaryStream),
          maxVertexValue,
          geometryColumnSorted);
    }
  }

  private static void addLineString(
      boolean containsPolygon, int numVertices, List<Integer> numParts, List<Integer> numRings) {
    /** Depending on the max geometry type in the column add to the numRings or numParts stream */
    if (containsPolygon) {
      numRings.add(numVertices);
    } else {
      numParts.add(numVertices);
    }
  }

  public static int[] zigZagDeltaEncodeVertices(Collection<Vertex> vertices) {
    Vertex previousVertex = new Vertex(0, 0);
    var deltaValues = new int[vertices.size() * 2];
    var j = 0;
    for (var vertex : vertices) {
      var delta = vertex.x() - previousVertex.x();
      var zigZagDelta = EncodingUtils.encodeZigZag(delta);
      deltaValues[j++] = zigZagDelta;

      delta = vertex.y() - previousVertex.y();
      zigZagDelta = EncodingUtils.encodeZigZag(delta);
      deltaValues[j++] = zigZagDelta;

      previousVertex = vertex;
    }

    return deltaValues;
  }

  private static List<Integer> getVertexOffsets(
      List<Vertex> vertexBuffer,
      Function<Integer, Integer> vertexOffsetSupplier,
      SpaceFillingCurve curve) {
    return vertexBuffer.stream()
        .map(
            vertex -> {
              var sfcId = curve.encode(vertex);
              return vertexOffsetSupplier.apply(sfcId);
            })
        .collect(Collectors.toList());
  }

  private static TreeMap<Integer, Vertex> addVerticesToDictionary(
      List<Vertex> vertices, HilbertCurve hilbertCurve) {
    var vertexDictionary = new TreeMap<Integer, Vertex>();
    for (var vertex : vertices) {
      var hilbertId = hilbertCurve.encode(vertex);
      vertexDictionary.put(hilbertId, vertex);
    }
    return vertexDictionary;
  }

  private static TreeSet<Integer> addVerticesToMortonDictionary(
      List<Vertex> vertices, ZOrderCurve zOrderCurve) {
    var mortonVertexDictionary = new TreeSet<Integer>();
    for (var vertex : vertices) {
      var mortonCode = zOrderCurve.encode(vertex);
      mortonVertexDictionary.add(mortonCode);
    }
    return mortonVertexDictionary;
  }

  private static List<Vertex> flatLineString(LineString lineString) {
    return Arrays.stream(lineString.getCoordinates())
        .map(v -> new Vertex((int) v.x, (int) v.y))
        .collect(Collectors.toList());
  }

  private static List<Vertex> flatPolygon(
      Polygon polygon, List<Integer> partSize, List<Integer> ringSize) {
    var numRings = polygon.getNumInteriorRing() + 1;
    partSize.add(numRings);

    var exteriorRing = polygon.getExteriorRing();
    var shell =
        new GeometryFactory()
            .createLineString(
                Arrays.copyOf(
                    exteriorRing.getCoordinates(), exteriorRing.getCoordinates().length - 1));
    var shellVertices = flatLineString(shell);
    var vertexBuffer = new ArrayList<>(shellVertices);
    ringSize.add(shell.getNumPoints());

    for (var i = 0; i < polygon.getNumInteriorRing(); i++) {
      var interiorRing = polygon.getInteriorRingN(i);
      var ring =
          new GeometryFactory()
              .createLineString(
                  Arrays.copyOf(
                      interiorRing.getCoordinates(), interiorRing.getCoordinates().length - 1));

      var ringVertices = flatLineString(ring);
      vertexBuffer.addAll(ringVertices);
      ringSize.add(ring.getNumPoints());
    }

    return vertexBuffer;
  }

  /**
   * Encodes the StreamMetadata and applies the specified physical level technique to the values.
   */
  private static byte[] encodeVertexBuffer(
      List<Integer> values, PhysicalLevelTechnique physicalLevelTechnique) {
    var encodedValues =
        physicalLevelTechnique == PhysicalLevelTechnique.FAST_PFOR
            ? encodeFastPfor(values, false)
            : encodeVarint(
                values.stream().mapToLong(i -> i).boxed().collect(Collectors.toList()), false);

    var encodedMetadata =
        new StreamMetadata(
                PhysicalStreamType.DATA,
                new LogicalStreamType(DictionaryType.VERTEX),
                LogicalLevelTechnique.COMPONENTWISE_DELTA,
                LogicalLevelTechnique.NONE,
                physicalLevelTechnique,
                values.size(),
                encodedValues.length)
            .encode();

    return ArrayUtils.addAll(encodedMetadata, encodedValues);
  }
}
