package org.maplibre.mlt.converter.encodings;

import static org.maplibre.mlt.converter.encodings.IntegerEncoder.encodeFastPfor;
import static org.maplibre.mlt.converter.encodings.IntegerEncoder.encodeVarint;

import com.carrotsearch.hppc.IntArrayList;
import jakarta.annotation.Nullable;
import java.io.IOException;
import java.net.URI;
import java.util.*;
import java.util.function.Function;
import java.util.stream.Stream;
import org.apache.commons.lang3.ArrayUtils;
import org.apache.commons.lang3.tuple.Pair;
import org.jetbrains.annotations.NotNull;
import org.locationtech.jts.geom.Geometry;
import org.locationtech.jts.geom.GeometryFactory;
import org.locationtech.jts.geom.LineString;
import org.locationtech.jts.geom.LinearRing;
import org.locationtech.jts.geom.MultiLineString;
import org.locationtech.jts.geom.MultiPoint;
import org.locationtech.jts.geom.MultiPolygon;
import org.locationtech.jts.geom.Point;
import org.locationtech.jts.geom.Polygon;
import org.maplibre.mlt.converter.CollectionUtils;
import org.maplibre.mlt.converter.ConversionConfig.IntegerEncodingOption;
import org.maplibre.mlt.converter.MLTStreamObserver;
import org.maplibre.mlt.converter.geometry.*;
import org.maplibre.mlt.converter.tessellation.TessellationUtils;
import org.maplibre.mlt.metadata.stream.*;

public class GeometryEncoder {

  public record EncodedGeometryColumn(
      int numStreams, byte[] encodedValues, int maxVertexValue, boolean geometryColumnSorted) {}

  public record SortSettings(boolean isSortable, List<Long> featureIds) {}

  private GeometryEncoder() {}

  /**
   * Backward-compatible overload that uses {@link IntegerEncodingOption#AUTO} for geometry stream
   * encoding. Prefer {@link #encodePretessellatedGeometryColumn(List, PhysicalLevelTechnique,
   * SortSettings, boolean, boolean, URI, IntegerEncodingOption, MLTStreamObserver)} when you need
   * to control encoding.
   */
  public static EncodedGeometryColumn encodePretessellatedGeometryColumn(
      List<Geometry> geometries,
      PhysicalLevelTechnique physicalLevelTechnique,
      SortSettings sortSettings,
      boolean useMortonEncoding,
      boolean encodePolygonOutlines,
      @Nullable URI tessellateSource,
      @NotNull MLTStreamObserver streamObserver)
      throws IOException {
    return encodePretessellatedGeometryColumn(
        geometries,
        physicalLevelTechnique,
        sortSettings,
        useMortonEncoding,
        encodePolygonOutlines,
        tessellateSource,
        IntegerEncodingOption.AUTO,
        streamObserver);
  }

  public static EncodedGeometryColumn encodePretessellatedGeometryColumn(
      List<Geometry> geometries,
      PhysicalLevelTechnique physicalLevelTechnique,
      SortSettings sortSettings,
      boolean useMortonEncoding,
      boolean encodePolygonOutlines,
      @Nullable URI tessellateSource,
      @NotNull IntegerEncodingOption encodingOption,
      @NotNull MLTStreamObserver streamObserver)
      throws IOException {
    final var geometryTypes = new ArrayList<Integer>();
    final var numGeometries = new ArrayList<Integer>();
    final var numParts = new ArrayList<Integer>();
    final var numRings = new ArrayList<Integer>();
    final var numTriangles = new ArrayList<Integer>();
    final var indexBuffer = new ArrayList<Integer>();
    final var vertexBuffer = new ArrayList<Vertex>();
    final var containsPolygon = containsPolygon(geometries);
    for (var geometry : geometries) {
      switch (geometry) {
        case Point point -> {
          geometryTypes.add(GeometryType.POINT.ordinal());
          var x = (int) point.getX();
          var y = (int) point.getY();
          vertexBuffer.add(new Vertex(x, y));
        }
        case LineString lineString -> {
          // TODO: verify if part of a MultiPolygon or Polygon geometry add then to numRings?
          geometryTypes.add(GeometryType.LINESTRING.ordinal());
          var numVertices = lineString.getCoordinates().length;
          addLineString(containsPolygon, numVertices, numParts, numRings);
          var vertices = flatLineString(lineString);
          vertexBuffer.addAll(vertices);
        }
        case Polygon polygon -> {
          geometryTypes.add(GeometryType.POLYGON.ordinal());
          flatPolygon(polygon, vertexBuffer, numParts, numRings);

          var tessellatedPolygon =
              TessellationUtils.tessellatePolygon(polygon, 0, tessellateSource);
          numTriangles.add(tessellatedPolygon.numTriangles());
          indexBuffer.addAll(tessellatedPolygon.indexBuffer());
        }
        case MultiLineString multiLineString -> {
          // TODO: verify if part of a MultiPolygon or Polygon geometry add then to numRings?
          geometryTypes.add(GeometryType.MULTILINESTRING.ordinal());
          var numLineStrings = multiLineString.getNumGeometries();
          numGeometries.add(numLineStrings);
          for (var i = 0; i < numLineStrings; i++) {
            var lineString = (LineString) multiLineString.getGeometryN(i);
            var numVertices = lineString.getCoordinates().length;
            addLineString(containsPolygon, numVertices, numParts, numRings);
            vertexBuffer.addAll(flatLineString(lineString));
          }
        }
        case MultiPolygon multiPolygon -> {
          geometryTypes.add(GeometryType.MULTIPOLYGON.ordinal());
          var numPolygons = multiPolygon.getNumGeometries();
          numGeometries.add(numPolygons);
          for (var i = 0; i < numPolygons; i++) {
            var polygon = (Polygon) multiPolygon.getGeometryN(i);
            flatPolygon(polygon, vertexBuffer, numParts, numRings);
          }

          // TODO: use also a vertex dictionary encoding for MultiPolygon geometries
          var tessellatedPolygon =
              TessellationUtils.tessellateMultiPolygon(multiPolygon, tessellateSource);
          numTriangles.add(tessellatedPolygon.numTriangles());
          indexBuffer.addAll(tessellatedPolygon.indexBuffer());
        }
        case MultiPoint multiPoint -> {
          geometryTypes.add(GeometryType.MULTIPOINT.ordinal());
          var numPoints = multiPoint.getNumGeometries();
          numGeometries.add(numPoints);
          for (var i = 0; i < numPoints; i++) {
            final var point = (Point) multiPoint.getGeometryN(i);
            final var x = (int) point.getX();
            final var y = (int) point.getY();
            vertexBuffer.add(new Vertex(x, y));
          }
        }
        default ->
            throw new IllegalArgumentException(
                "Specified geometry type is not (yet) supported: " + geometry.getGeometryType());
      }
    }

    if (vertexBuffer.isEmpty()) {
      throw new IllegalArgumentException("The geometry column contains no vertices");
    }

    // TODO: get rid of that separate calculation
    var minVertexValue = Integer.MAX_VALUE;
    var maxVertexValue = Integer.MIN_VALUE;
    for (var vertex : vertexBuffer) {
      if (vertex.x() < minVertexValue) minVertexValue = vertex.x();
      if (vertex.y() < minVertexValue) minVertexValue = vertex.y();
      if (vertex.x() > maxVertexValue) maxVertexValue = vertex.x();
      if (vertex.y() > maxVertexValue) maxVertexValue = vertex.y();
    }

    final var hilbertCurve = new HilbertCurve(minVertexValue, maxVertexValue);
    final var zOrderCurve = new ZOrderCurve(minVertexValue, maxVertexValue);
    // TODO: if the ratio is lower than 2 dictionary encoding has not to be considered?
    var vertexDictionary = addVerticesToDictionary(vertexBuffer, hilbertCurve);
    var mortonEncodedDictionary = addVerticesToMortonDictionary(vertexBuffer, zOrderCurve);

    var dictionaryOffsets =
        getVertexOffsets(vertexBuffer, reverseMap(vertexDictionary.getLeft())::get, hilbertCurve);

    var mortonEncodedDictionaryOffsets =
        getVertexOffsets(vertexBuffer, reverseMap(mortonEncodedDictionary)::get, zOrderCurve);

    /* Test if Plain, Vertex Dictionary or Morton Encoded Vertex Dictionary is the most efficient
     * -> Plain -> convert VertexBuffer with Delta Encoding and specified Physical Level Technique
     * -> Dictionary -> convert VertexOffsets with IntegerEncoder and VertexBuffer with Delta Encoding and specified Physical Level Technique
     * -> Morton Encoded Dictionary -> convert VertexOffsets with Integer Encoder and VertexBuffer with IntegerEncoder
     * */
    var zigZagDeltaVertexBuffer = zigZagDeltaEncodeVertices(vertexBuffer);
    var zigZagDeltaVertexDictionary = zigZagDeltaEncodeVertices(vertexDictionary.getRight());

    // TODO: get rid of that conversions
    // TODO: should we do a potential recursive encoding again
    var encodedVertexBuffer =
        IntegerEncoder.encodeInt(
            zigZagDeltaVertexBuffer, physicalLevelTechnique, false, encodingOption);
    // TODO: should we do a potential recursive encoding again
    var encodedVertexDictionary =
        IntegerEncoder.encodeInt(
            zigZagDeltaVertexDictionary, physicalLevelTechnique, false, encodingOption);
    var encodedMortonVertexDictionary =
        IntegerEncoder.encodeMortonCodes(mortonEncodedDictionary, physicalLevelTechnique);
    var encodedDictionaryOffsets =
        IntegerEncoder.encodeInt(dictionaryOffsets, physicalLevelTechnique, false, encodingOption);
    var encodedMortonEncodedDictionaryOffsets =
        IntegerEncoder.encodeInt(
            mortonEncodedDictionaryOffsets, physicalLevelTechnique, false, encodingOption);

    // TODO: refactor this simple approach to also work with mixed geometries
    var geometryColumnSorted = false;
    if (sortSettings.isSortable && numGeometries.isEmpty() && numRings.isEmpty()) {
      if (numParts.size() == sortSettings.featureIds.size()) {
        /* Currently the VertexOffsets are only sorted if all geometries in the geometry column are of type LineString */
        GeometryUtils.sortVertexOffsets(
            numParts, mortonEncodedDictionaryOffsets, sortSettings.featureIds());
        encodedMortonEncodedDictionaryOffsets =
            IntegerEncoder.encodeInt(
                mortonEncodedDictionaryOffsets, physicalLevelTechnique, false, encodingOption);
        geometryColumnSorted = true;
      } else if (numParts.isEmpty()) {
        GeometryUtils.sortPoints(vertexBuffer, hilbertCurve, sortSettings.featureIds);
        zigZagDeltaVertexBuffer = zigZagDeltaEncodeVertices(vertexBuffer);
        encodedVertexBuffer =
            IntegerEncoder.encodeInt(
                zigZagDeltaVertexBuffer, physicalLevelTechnique, false, encodingOption);
        geometryColumnSorted = true;
      }
    }

    var encodedGeometryTypesStream =
        IntegerEncoder.encodeIntStream(
            geometryTypes,
            physicalLevelTechnique,
            false,
            PhysicalStreamType.LENGTH,
            null,
            encodingOption,
            streamObserver,
            "geom_types");
    var encodedTopologyStreams = encodedGeometryTypesStream;
    var numStreams = 1;

    if (!numGeometries.isEmpty()) {
      var encodedNumGeometries =
          IntegerEncoder.encodeIntStream(
              numGeometries,
              physicalLevelTechnique,
              false,
              PhysicalStreamType.LENGTH,
              new LogicalStreamType(LengthType.GEOMETRIES),
              encodingOption,
              streamObserver,
              "geom_num_geoms");
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
              new LogicalStreamType(LengthType.PARTS),
              encodingOption,
              streamObserver,
              "geom_num_parts");
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
              new LogicalStreamType(LengthType.RINGS),
              encodingOption,
              streamObserver,
              "geom_num_rings");
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

    // TODO: move pre-tessellation column creation up to avoid doing unnecessary work
    /* Currently use pre-tessellation only if all geometries in a FeatureTable are Polygons or MultiPolygons */
    final boolean includePreTessellatedPolygonGeometry = containsOnlyPolygons(geometryTypes);

    if (includePreTessellatedPolygonGeometry) {
      // TODO: also support Vertex Dictionary and Morton Encoded Vertex Dictionary encoding?
      var encodedVertexBufferStream =
          encodeVertexBuffer(
              zigZagDeltaVertexBuffer, vertexBuffer, physicalLevelTechnique, streamObserver);

      if (encodePolygonOutlines) {
        var encodedPretessellationStreams =
            encodePolygonPretessellationStreamsWithOutlines(
                physicalLevelTechnique,
                encodingOption,
                numGeometries,
                numParts,
                numRings,
                numTriangles,
                indexBuffer,
                streamObserver);
        return new EncodedGeometryColumn(
            7,
            CollectionUtils.concatByteArrays(
                encodedGeometryTypesStream,
                encodedPretessellationStreams,
                encodedVertexBufferStream),
            maxVertexValue,
            geometryColumnSorted);
      }

      var encodedPretessellationStreams =
          encodePolygonPretessellationStreams(
              physicalLevelTechnique, encodingOption, numTriangles, indexBuffer, streamObserver);
      return new EncodedGeometryColumn(
          4,
          CollectionUtils.concatByteArrays(
              encodedGeometryTypesStream, encodedPretessellationStreams, encodedVertexBufferStream),
          maxVertexValue,
          geometryColumnSorted);
    } else if (plainVertexBufferSize <= dictionaryEncodedSize
        && plainVertexBufferSize <= mortonDictionaryEncodedSize) {
      // TODO: get rid of extra conversion
      var encodedVertexBufferStream =
          encodeVertexBuffer(
              zigZagDeltaVertexBuffer, vertexBuffer, physicalLevelTechnique, streamObserver);

      return new EncodedGeometryColumn(
          numStreams + 1,
          ArrayUtils.addAll(encodedTopologyStreams, encodedVertexBufferStream),
          maxVertexValue,
          geometryColumnSorted);
    } else if ((dictionaryEncodedSize < plainVertexBufferSize
            && dictionaryEncodedSize <= mortonDictionaryEncodedSize)
        || !useMortonEncoding) {
      var encodedVertexOffsetStream =
          IntegerEncoder.encodeIntStream(
              dictionaryOffsets,
              physicalLevelTechnique,
              false,
              PhysicalStreamType.OFFSET,
              new LogicalStreamType(OffsetType.VERTEX),
              encodingOption,
              streamObserver,
              "geom_vertex_offsets");

      var encodedVertexDictionaryStream =
          encodeVertexBuffer(
              zigZagDeltaVertexDictionary,
              vertexDictionary.getRight(),
              physicalLevelTechnique,
              streamObserver);

      return new EncodedGeometryColumn(
          numStreams + 2,
          CollectionUtils.concatByteArrays(
              encodedTopologyStreams, encodedVertexOffsetStream, encodedVertexDictionaryStream),
          maxVertexValue,
          false);
    }
    // TODO: add morton again
    else {
      // Note: input values are morton-encoded as they're produced, so the values here are not the
      // raw values
      var encodedMortonVertexOffsetStream =
          IntegerEncoder.encodeIntStream(
              mortonEncodedDictionaryOffsets,
              physicalLevelTechnique,
              false,
              PhysicalStreamType.OFFSET,
              new LogicalStreamType(OffsetType.VERTEX),
              encodingOption,
              streamObserver,
              "geom_morton_vertex_offsets");

      var encodedMortonEncodedVertexDictionaryStream =
          IntegerEncoder.encodeMortonStream(
              mortonEncodedDictionary,
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

  private static byte[] encodePolygonPretessellationStreams(
      PhysicalLevelTechnique physicalLevelTechnique,
      @NotNull IntegerEncodingOption encodingOption,
      ArrayList<Integer> numTriangles,
      ArrayList<Integer> indexBuffer,
      @NotNull MLTStreamObserver streamObserver)
      throws IOException {
    var encodedNumTrianglesBuffer =
        IntegerEncoder.encodeIntStream(
            numTriangles,
            physicalLevelTechnique,
            false,
            PhysicalStreamType.LENGTH,
            new LogicalStreamType(LengthType.TRIANGLES),
            encodingOption,
            streamObserver,
            "geom_num_tris");
    var encodedIndexBuffer =
        IntegerEncoder.encodeIntStream(
            indexBuffer,
            physicalLevelTechnique,
            false,
            PhysicalStreamType.OFFSET,
            new LogicalStreamType(OffsetType.INDEX),
            encodingOption,
            streamObserver,
            "geom_indexes");

    return ArrayUtils.addAll(encodedNumTrianglesBuffer, encodedIndexBuffer);
  }

  private static byte[] encodePolygonPretessellationStreamsWithOutlines(
      PhysicalLevelTechnique physicalLevelTechnique,
      @NotNull IntegerEncodingOption encodingOption,
      ArrayList<Integer> numGeometries,
      ArrayList<Integer> numParts,
      ArrayList<Integer> numRings,
      ArrayList<Integer> numTriangles,
      ArrayList<Integer> indexBuffer,
      @NotNull MLTStreamObserver streamObserver)
      throws IOException {
    var encodedNumGeometries =
        IntegerEncoder.encodeIntStream(
            numGeometries,
            physicalLevelTechnique,
            false,
            PhysicalStreamType.LENGTH,
            new LogicalStreamType(LengthType.GEOMETRIES),
            encodingOption,
            streamObserver,
            "geom_num_geoms");
    var encodedNumParts =
        IntegerEncoder.encodeIntStream(
            numParts,
            physicalLevelTechnique,
            false,
            PhysicalStreamType.LENGTH,
            new LogicalStreamType(LengthType.PARTS),
            encodingOption,
            streamObserver,
            "geom_num_parts");
    var encodedNumRings =
        IntegerEncoder.encodeIntStream(
            numRings,
            physicalLevelTechnique,
            false,
            PhysicalStreamType.LENGTH,
            new LogicalStreamType(LengthType.RINGS),
            encodingOption,
            streamObserver,
            "geom_num_rings");
    var encodedNumTrianglesBuffer =
        IntegerEncoder.encodeIntStream(
            numTriangles,
            physicalLevelTechnique,
            false,
            PhysicalStreamType.LENGTH,
            new LogicalStreamType(LengthType.TRIANGLES),
            encodingOption,
            streamObserver,
            "geom_num_tris");
    var encodedIndexBuffer =
        IntegerEncoder.encodeIntStream(
            indexBuffer,
            physicalLevelTechnique,
            false,
            PhysicalStreamType.OFFSET,
            new LogicalStreamType(OffsetType.INDEX),
            encodingOption,
            streamObserver,
            "geom_indexes");

    return CollectionUtils.concatByteArrays(
        encodedNumGeometries,
        encodedNumParts,
        encodedNumRings,
        encodedNumTrianglesBuffer,
        encodedIndexBuffer);
  }

  private static boolean containsPolygon(List<Geometry> geometries) {
    return geometries.stream()
        .map(Geometry::getGeometryType)
        .anyMatch(
            t -> t.equals(Geometry.TYPENAME_MULTIPOLYGON) || t.equals(Geometry.TYPENAME_POLYGON));
  }

  public static boolean containsOnlyPolygons(List<Integer> geometryTypes) {
    return geometryTypes.stream()
        .allMatch(
            geometryType ->
                geometryType == GeometryType.POLYGON.ordinal()
                    || geometryType == GeometryType.MULTIPOLYGON.ordinal());
  }

  /**
   * Backward-compatible overload that uses {@link IntegerEncodingOption#AUTO} for geometry stream
   * encoding. Prefer {@link #encodeGeometryColumn(List, PhysicalLevelTechnique, SortSettings,
   * boolean, IntegerEncodingOption, MLTStreamObserver)} when you need to control encoding.
   */
  public static EncodedGeometryColumn encodeGeometryColumn(
      List<Geometry> geometries,
      PhysicalLevelTechnique physicalLevelTechnique,
      SortSettings sortSettings,
      boolean useMortonEncoding,
      @NotNull MLTStreamObserver streamObserver)
      throws IOException {
    return encodeGeometryColumn(
        geometries,
        physicalLevelTechnique,
        sortSettings,
        useMortonEncoding,
        IntegerEncodingOption.AUTO,
        streamObserver);
  }

  // TODO: add selection algorithms based on statistics and sampling
  public static EncodedGeometryColumn encodeGeometryColumn(
      List<Geometry> geometries,
      PhysicalLevelTechnique physicalLevelTechnique,
      SortSettings sortSettings,
      boolean useMortonEncoding,
      @NotNull IntegerEncodingOption encodingOption,
      @NotNull MLTStreamObserver streamObserver)
      throws IOException {
    var geometryTypes = new ArrayList<Integer>();
    var numGeometries = new ArrayList<Integer>();
    var numParts = new ArrayList<Integer>();
    var numRings = new ArrayList<Integer>();
    var vertexBuffer = new ArrayList<Vertex>();
    final var containsPolygon = containsPolygon(geometries);
    for (var geometry : geometries) {
      switch (geometry) {
        case Point point -> {
          geometryTypes.add(GeometryType.POINT.ordinal());
          vertexBuffer.add(new Vertex((int) point.getX(), (int) point.getY()));
        }
        case LineString lineString -> {
          // TODO: verify if part of a MultiPolygon or Polygon geometry add then to numRings?
          geometryTypes.add(GeometryType.LINESTRING.ordinal());
          var numVertices = lineString.getCoordinates().length;
          addLineString(containsPolygon, numVertices, numParts, numRings);
          var vertices = flatLineString(lineString);
          vertexBuffer.addAll(vertices);
        }
        case Polygon polygon -> {
          geometryTypes.add(GeometryType.POLYGON.ordinal());
          flatPolygon(polygon, vertexBuffer, numParts, numRings);
        }
        case MultiLineString multiLineString -> {
          // TODO: verify if part of a MultiPolygon or Polygon geometry add then to numRings?
          geometryTypes.add(GeometryType.MULTILINESTRING.ordinal());
          var numLineStrings = multiLineString.getNumGeometries();
          numGeometries.add(numLineStrings);
          for (var i = 0; i < numLineStrings; i++) {
            var lineString = (LineString) multiLineString.getGeometryN(i);
            var numVertices = lineString.getCoordinates().length;
            addLineString(containsPolygon, numVertices, numParts, numRings);
            vertexBuffer.addAll(flatLineString(lineString));
          }
        }
        case MultiPolygon multiPolygon -> {
          geometryTypes.add(GeometryType.MULTIPOLYGON.ordinal());
          var numPolygons = multiPolygon.getNumGeometries();
          numGeometries.add(numPolygons);
          for (var i = 0; i < numPolygons; i++) {
            var polygon = (Polygon) multiPolygon.getGeometryN(i);
            flatPolygon(polygon, vertexBuffer, numParts, numRings);
          }
        }
        case MultiPoint multiPoint -> {
          geometryTypes.add(GeometryType.MULTIPOINT.ordinal());
          var numPoints = multiPoint.getNumGeometries();
          numGeometries.add(numPoints);
          for (var i = 0; i < numPoints; i++) {
            var point = (Point) multiPoint.getGeometryN(i);
            var x = (int) point.getX();
            var y = (int) point.getY();
            vertexBuffer.add(new Vertex(x, y));
          }
        }
        default ->
            throw new IllegalArgumentException(
                "Specified geometry type is not (yet) supported: " + geometry.getGeometryType());
      }
    }

    if (vertexBuffer.isEmpty()) {
      throw new IllegalArgumentException("The geometry column contains no vertices");
    }

    var minVertexValue = Integer.MAX_VALUE;
    var maxVertexValue = Integer.MIN_VALUE;
    for (var vertex : vertexBuffer) {
      if (vertex.x() < minVertexValue) minVertexValue = vertex.x();
      if (vertex.y() < minVertexValue) minVertexValue = vertex.y();
      if (vertex.x() > maxVertexValue) maxVertexValue = vertex.x();
      if (vertex.y() > maxVertexValue) maxVertexValue = vertex.y();
    }

    var hilbertCurve = new HilbertCurve(minVertexValue, maxVertexValue);
    var zOrderCurve = new ZOrderCurve(minVertexValue, maxVertexValue);
    // TODO: if the ratio is lower then 2 dictionary encoding has not to be considered?
    var vertexDictionary = addVerticesToDictionary(vertexBuffer, hilbertCurve);
    var mortonEncodedDictionary = addVerticesToMortonDictionary(vertexBuffer, zOrderCurve);

    var dictionaryOffsets =
        getVertexOffsets(vertexBuffer, reverseMap(vertexDictionary.getLeft())::get, hilbertCurve);

    var mortonEncodedDictionaryOffsets =
        getVertexOffsets(vertexBuffer, reverseMap(mortonEncodedDictionary)::get, zOrderCurve);

    /* Test if Plain, Vertex Dictionary or Morton Encoded Vertex Dictionary is the most efficient
     * -> Plain -> convert VertexBuffer with Delta Encoding and specified Physical Level Technique
     * -> Dictionary -> convert VertexOffsets with IntegerEncoder and VertexBuffer with Delta Encoding and specified Physical Level Technique
     * -> Morton Encoded Dictionary -> convert VertexOffsets with Integer Encoder and VertexBuffer with IntegerEncoder
     * */
    var zigZagDeltaVertexBuffer = zigZagDeltaEncodeVertices(vertexBuffer);
    var zigZagDeltaVertexDictionary = zigZagDeltaEncodeVertices(vertexDictionary.getRight());

    // TODO: get rid of that conversions
    // TODO: should we do a potential recursive encoding again
    var encodedVertexBuffer =
        IntegerEncoder.encodeInt(
            zigZagDeltaVertexBuffer, physicalLevelTechnique, false, encodingOption);
    // TODO: should we do a potential recursive encoding again
    var encodedVertexDictionary =
        IntegerEncoder.encodeInt(
            zigZagDeltaVertexDictionary, physicalLevelTechnique, false, encodingOption);
    var encodedMortonVertexDictionary =
        IntegerEncoder.encodeMortonCodes(mortonEncodedDictionary, physicalLevelTechnique);
    var encodedDictionaryOffsets =
        IntegerEncoder.encodeInt(dictionaryOffsets, physicalLevelTechnique, false, encodingOption);
    var encodedMortonEncodedDictionaryOffsets =
        IntegerEncoder.encodeInt(
            mortonEncodedDictionaryOffsets, physicalLevelTechnique, false, encodingOption);

    // TODO: refactor this simple approach to also work with mixed geometries
    var geometryColumnSorted = false;
    if (sortSettings.isSortable && numGeometries.isEmpty() && numRings.isEmpty()) {
      if (numParts.size() == sortSettings.featureIds.size()) {
        /* Currently the VertexOffsets are only sorted if all geometries in the geometry column are of type LineString */
        GeometryUtils.sortVertexOffsets(
            numParts, mortonEncodedDictionaryOffsets, sortSettings.featureIds());
        encodedMortonEncodedDictionaryOffsets =
            IntegerEncoder.encodeInt(
                mortonEncodedDictionaryOffsets, physicalLevelTechnique, false, encodingOption);
        geometryColumnSorted = true;
      } else if (numParts.isEmpty()) {
        GeometryUtils.sortPoints(vertexBuffer, hilbertCurve, sortSettings.featureIds);
        zigZagDeltaVertexBuffer = zigZagDeltaEncodeVertices(vertexBuffer);
        encodedVertexBuffer =
            IntegerEncoder.encodeInt(
                zigZagDeltaVertexBuffer, physicalLevelTechnique, false, encodingOption);
        geometryColumnSorted = true;
      }
    }

    var encodedTopologyStreams =
        IntegerEncoder.encodeIntStream(
            geometryTypes,
            physicalLevelTechnique,
            false,
            PhysicalStreamType.LENGTH,
            null,
            encodingOption,
            streamObserver,
            "geom_types");
    var numStreams = 1;

    if (!numGeometries.isEmpty()) {
      var encodedNumGeometries =
          IntegerEncoder.encodeIntStream(
              numGeometries,
              physicalLevelTechnique,
              false,
              PhysicalStreamType.LENGTH,
              new LogicalStreamType(LengthType.GEOMETRIES),
              encodingOption,
              streamObserver,
              "geom_num_geoms");
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
              new LogicalStreamType(LengthType.PARTS),
              encodingOption,
              streamObserver,
              "geom_num_parts");
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
              new LogicalStreamType(LengthType.RINGS),
              encodingOption,
              streamObserver,
              "geom_num_rings");
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
        && (!useMortonEncoding || plainVertexBufferSize <= mortonDictionaryEncodedSize)) {
      // TODO: get rid of extra conversion
      var encodedVertexBufferStream =
          encodeVertexBuffer(
              zigZagDeltaVertexBuffer, vertexBuffer, physicalLevelTechnique, streamObserver);

      return new EncodedGeometryColumn(
          numStreams + 1,
          ArrayUtils.addAll(encodedTopologyStreams, encodedVertexBufferStream),
          maxVertexValue,
          geometryColumnSorted);
    } else if (dictionaryEncodedSize < plainVertexBufferSize
        && (!useMortonEncoding || dictionaryEncodedSize <= mortonDictionaryEncodedSize)) {
      var encodedVertexOffsetStream =
          IntegerEncoder.encodeIntStream(
              dictionaryOffsets,
              physicalLevelTechnique,
              false,
              PhysicalStreamType.OFFSET,
              new LogicalStreamType(OffsetType.VERTEX),
              encodingOption,
              streamObserver,
              "geom_vertex_offsets");
      var encodedVertexDictionaryStream =
          encodeVertexBuffer(
              zigZagDeltaVertexDictionary,
              vertexDictionary.getRight(),
              physicalLevelTechnique,
              streamObserver);

      return new EncodedGeometryColumn(
          numStreams + 2,
          CollectionUtils.concatByteArrays(
              encodedTopologyStreams, encodedVertexOffsetStream, encodedVertexDictionaryStream),
          maxVertexValue,
          false);
    } else {
      // Note: input values are morton-encoded as they're produced, so the values here are not the
      // raw values
      var encodedMortonVertexOffsetStream =
          IntegerEncoder.encodeIntStream(
              mortonEncodedDictionaryOffsets,
              physicalLevelTechnique,
              false,
              PhysicalStreamType.OFFSET,
              new LogicalStreamType(OffsetType.VERTEX),
              encodingOption,
              streamObserver,
              "geom_morton_vertex_offsets");

      var encodedMortonEncodedVertexDictionaryStream =
          IntegerEncoder.encodeMortonStream(
              mortonEncodedDictionary,
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
    /* Depending on the max geometry type in the column add to the numRings or numParts stream */
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

  private static int[] getVertexOffsets(
      List<Vertex> vertexBuffer,
      Function<Integer, Integer> vertexOffsetSupplier,
      SpaceFillingCurve curve) {
    int[] result = new int[vertexBuffer.size()];
    int i = 0;
    for (var vertex : vertexBuffer) {
      result[i++] = vertexOffsetSupplier.apply(curve.encode(vertex));
    }
    return result;
  }

  private static Map<Integer, Integer> reverseMap(Collection<Integer> mortonEncodedDictionary) {
    Map<Integer, Integer> morton = HashMap.newHashMap(mortonEncodedDictionary.size());
    int i = 0;
    for (var item : mortonEncodedDictionary) {
      morton.put(item, i++);
    }
    return morton;
  }

  private static Map<Integer, Integer> reverseMap(int[] mortonEncodedDictionary) {
    Map<Integer, Integer> morton = HashMap.newHashMap(mortonEncodedDictionary.length);
    int i = 0;
    for (var item : mortonEncodedDictionary) {
      morton.put(item, i++);
    }
    return morton;
  }

  record Indexed(int hilbert, Vertex vertex) implements Comparable<Indexed> {
    @Override
    public int compareTo(@NotNull GeometryEncoder.Indexed o) {
      return Integer.compare(hilbert, o.hilbert);
    }
  }

  private static Pair<List<Integer>, List<Vertex>> addVerticesToDictionary(
      List<Vertex> vertices, HilbertCurve hilbertCurve) {
    ArrayList<Indexed> vertexDictionary = new ArrayList<>();
    for (var vertex : vertices) {
      var hilbertId = hilbertCurve.encode(vertex);
      vertexDictionary.add(new Indexed(hilbertId, vertex));
    }
    vertexDictionary.sort(Comparator.naturalOrder());
    List<Integer> a = new ArrayList<>();
    List<Vertex> b = new ArrayList<>();
    int last = Integer.MIN_VALUE;
    for (var item : vertexDictionary) {
      if (item.hilbert != last) {
        a.add(item.hilbert);
        b.add(item.vertex);
      }
    }
    return Pair.of(a, b);
  }

  private static int[] addVerticesToMortonDictionary(
      List<Vertex> vertices, ZOrderCurve zOrderCurve) {
    IntArrayList result = new IntArrayList(vertices.size());
    for (var vertex : vertices) {
      var mortonCode = zOrderCurve.encode(vertex);
      result.add(mortonCode);
    }
    int[] resultArray = result.toArray();
    Arrays.sort(resultArray);
    return resultArray;
  }

  private static List<Vertex> flatLineString(LineString lineString) {
    return Arrays.stream(lineString.getCoordinates())
        .map(v -> new Vertex((int) v.x, (int) v.y))
        .toList();
  }

  private static LineString ringToLineString(LinearRing ring, GeometryFactory factory) {
    return factory.createLineString(
        Arrays.copyOf(ring.getCoordinates(), ring.getCoordinates().length - 1));
  }

  private static void flatPolygon(
      Polygon polygon, ArrayList<Vertex> vertices, List<Integer> partSize, List<Integer> ringSize) {
    final var factory = new GeometryFactory();

    // 1 for the outline, 1 for each interior ring
    partSize.add(1 + polygon.getNumInteriorRing());

    final var exteriorRing = polygon.getExteriorRing();
    // If the ring isn't closed, our assumptions about the number of vertices will be incorrect.
    assert (exteriorRing.isClosed());

    final var shell = ringToLineString(exteriorRing, factory);
    vertices.addAll(flatLineString(shell));
    ringSize.add(shell.getNumPoints());

    for (var i = 0; i < polygon.getNumInteriorRing(); i++) {
      final var interiorRing = polygon.getInteriorRingN(i);
      assert (interiorRing.isClosed());

      final var ring = ringToLineString(interiorRing, factory);
      vertices.addAll(flatLineString(ring));
      ringSize.add(ring.getNumPoints());
    }
  }

  /**
   * Encodes the StreamMetadata and applies the specified physical level technique to the values.
   */
  private static byte[] encodeVertexBuffer(
      int[] values,
      Collection<Vertex> vertices,
      PhysicalLevelTechnique physicalLevelTechnique,
      @NotNull MLTStreamObserver streamObserver)
      throws IOException {
    var encodedValues =
        physicalLevelTechnique == PhysicalLevelTechnique.FAST_PFOR
            ? encodeFastPfor(values, false)
            : encodeVarint(values, false);

    var encodedMetadata =
        new StreamMetadata(
                PhysicalStreamType.DATA,
                new LogicalStreamType(DictionaryType.VERTEX),
                LogicalLevelTechnique.COMPONENTWISE_DELTA,
                LogicalLevelTechnique.NONE,
                physicalLevelTechnique,
                values.length,
                encodedValues.length)
            .encode();

    if (streamObserver.isActive()) {
      final var rawValues = vertices.stream().flatMap(v -> Stream.of(v.x(), v.y())).toList();
      streamObserver.observeStream("geom_vertex_buffer", rawValues, encodedMetadata, encodedValues);
    }
    return ArrayUtils.addAll(encodedMetadata, encodedValues);
  }
}
