package org.maplibre.mlt.converter.encodings;

import static org.maplibre.mlt.converter.encodings.IntegerEncoder.encodeFastPfor;
import static org.maplibre.mlt.converter.encodings.IntegerEncoder.encodeVarint;

import jakarta.annotation.Nullable;
import java.io.IOException;
import java.net.URI;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collection;
import java.util.Comparator;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.function.Function;
import java.util.function.Predicate;
import java.util.stream.IntStream;
import java.util.stream.Stream;
import java.util.stream.StreamSupport;
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
import org.maplibre.mlt.converter.ConversionConfig.IntegerEncodingOption;
import org.maplibre.mlt.converter.geometry.GeometryType;
import org.maplibre.mlt.converter.geometry.GeometryUtils;
import org.maplibre.mlt.converter.geometry.HilbertCurve;
import org.maplibre.mlt.converter.geometry.SpaceFillingCurve;
import org.maplibre.mlt.converter.geometry.Vertex;
import org.maplibre.mlt.converter.geometry.ZOrderCurve;
import org.maplibre.mlt.converter.tessellation.TessellationUtils;
import org.maplibre.mlt.metadata.stream.DictionaryType;
import org.maplibre.mlt.metadata.stream.LengthType;
import org.maplibre.mlt.metadata.stream.LogicalLevelTechnique;
import org.maplibre.mlt.metadata.stream.LogicalStreamType;
import org.maplibre.mlt.metadata.stream.OffsetType;
import org.maplibre.mlt.metadata.stream.PhysicalLevelTechnique;
import org.maplibre.mlt.metadata.stream.PhysicalStreamType;
import org.maplibre.mlt.metadata.stream.StreamMetadata;

public class GeometryEncoder {

  public record EncodedGeometryColumn(
      int numStreams,
      ArrayList<byte[]> encodedValues,
      int maxVertexValue,
      boolean geometryColumnSorted) {}

  public record SortSettings(boolean isSortable, List<Long> featureIds) {}

  private GeometryEncoder() {}

  public static EncodedGeometryColumn encodeGeometryColumn(
      List<Geometry> geometries,
      PhysicalLevelTechnique physicalLevelTechnique,
      SortSettings sortSettings,
      boolean useMortonEncoding,
      boolean enableTessellation,
      boolean encodePolygonOutlines,
      @Nullable URI tessellateSource,
      @NotNull IntegerEncodingOption encodingOption)
      throws IOException {
    final var geometryTypes = new ArrayList<Integer>();
    final var numGeometries = new ArrayList<Integer>();
    final var numParts = new ArrayList<Integer>();
    final var numRings = new ArrayList<Integer>();
    final var numTriangles = enableTessellation ? new ArrayList<Integer>() : null;
    final var indexBuffer = enableTessellation ? new ArrayList<Integer>() : null;
    final var vertexBuffer = new ArrayList<Vertex>();
    prepareGeometry(
        geometries,
        numGeometries,
        geometryTypes,
        vertexBuffer,
        numParts,
        numRings,
        numTriangles,
        indexBuffer,
        tessellateSource);

    if (vertexBuffer.isEmpty()) {
      throw new IllegalArgumentException("The geometry column contains no vertices");
    }

    /* Test if Plain, Vertex Dictionary or Morton Encoded Vertex Dictionary is the most efficient
     * -> Plain -> convert VertexBuffer with Delta Encoding and specified Physical Level Technique
     * -> Dictionary -> convert VertexOffsets with IntegerEncoder and VertexBuffer with Delta Encoding and specified Physical Level Technique
     * -> Morton Encoded Dictionary -> convert VertexOffsets with Integer Encoder and VertexBuffer with IntegerEncoder
     * */

    final var vertexLimits = getVertexLimits(vertexBuffer);
    final var hilbertCurve = new HilbertCurve(vertexLimits.min, vertexLimits.max);
    final var zOrderCurve = new ZOrderCurve(vertexLimits.min, vertexLimits.max);

    // TODO: if the ratio is lower than 2 dictionary encoding has not to be considered?
    final var vertexDictionary = addVerticesToDictionary(vertexBuffer, hilbertCurve);
    final var vertexDictionaryHilbertIndexes = vertexDictionary.getLeft();
    final var vertexDictionaryHilbertMap = reverseMap(vertexDictionaryHilbertIndexes);
    final var vertexDictionaryVertices = vertexDictionary.getRight();
    final var vertexDictionaryOffsets =
        getVertexOffsets(vertexBuffer, vertexDictionaryHilbertMap::get, hilbertCurve);
    final var zigZagDeltaVertexDictionary = zigZagDeltaEncodeVertices(vertexDictionaryVertices);
    final var mortonEncodedDictionary =
        useMortonEncoding ? addVerticesToMortonDictionary(vertexBuffer, zOrderCurve) : null;
    final var mortonEncodedDictionaryOffsets =
        useMortonEncoding
            ? getVertexOffsets(vertexBuffer, reverseMap(mortonEncodedDictionary)::get, zOrderCurve)
            : null;

    // TODO: refactor this simple approach to also work with mixed geometries
    var geometryColumnSorted = false;
    if (sortSettings.isSortable && numGeometries.isEmpty() && numRings.isEmpty()) {
      if (numParts.size() == sortSettings.featureIds.size()) {
        /* Currently the VertexOffsets are only sorted if all geometries in the geometry column are of type LineString */
        GeometryUtils.sortVertexOffsets(
            numParts, mortonEncodedDictionaryOffsets, sortSettings.featureIds());
        geometryColumnSorted = true;
      } else if (numParts.isEmpty()) {
        GeometryUtils.sortPoints(vertexBuffer, hilbertCurve, sortSettings.featureIds);
        geometryColumnSorted = true;
      }
    }

    final var zigZagDeltaVertexBuffer = zigZagDeltaEncodeVertices(vertexBuffer);
    final var encodedVertexBufferStream =
        encodeVertexBuffer(zigZagDeltaVertexBuffer, physicalLevelTechnique);

    // TODO: All of these are done only to determine which encoding to use, the actual result is
    //       discarded!  Additionally, it's only the size of the raw data, not including metadata.
    //       Instead, we should select based on the size of what's actually written.
    final var plainVertexBufferSize =
        IntegerEncoder.encodeInt(
                zigZagDeltaVertexBuffer, physicalLevelTechnique, false, encodingOption)
            .encodedValues
            .length;
    final var encodedMortonEncodedDictionaryOffsetsSize =
        useMortonEncoding
            ? IntegerEncoder.encodeInt(
                    mortonEncodedDictionaryOffsets, physicalLevelTechnique, false, encodingOption)
                .encodedValues
                .length
            : 0;
    final var encodedDictionaryOffsetsSize =
        IntegerEncoder.encodeInt(
                vertexDictionaryOffsets, physicalLevelTechnique, false, encodingOption)
            .encodedValues
            .length;
    final var encodedVertexDictionarySize =
        IntegerEncoder.encodeInt(
                zigZagDeltaVertexDictionary, physicalLevelTechnique, false, encodingOption)
            .encodedValues
            .length;
    final var encodedMortonVertexDictionarySize =
        useMortonEncoding
            ? IntegerEncoder.encodeMortonCodes(mortonEncodedDictionary, physicalLevelTechnique)
                .encodedValues
                .length
            : 0;
    final var dictionaryEncodedSize = encodedDictionaryOffsetsSize + encodedVertexDictionarySize;
    final var mortonDictionaryEncodedSize =
        encodedMortonEncodedDictionaryOffsetsSize + encodedMortonVertexDictionarySize;
    // TODO: end

    final var result =
        IntegerEncoder.encodeIntStream(
            geometryTypes,
            physicalLevelTechnique,
            false,
            PhysicalStreamType.LENGTH,
            null,
            encodingOption);
    var numStreams = 1;

    /* Currently use pre-tessellation only if all geometries in a FeatureTable are Polygons or MultiPolygons */
    if (enableTessellation && containsOnlyPolygons(geometryTypes)) {
      // TODO: also support Vertex Dictionary and Morton Encoded Vertex Dictionary encoding?
      numStreams +=
          encodePolygonPretessellationStreams(
              result,
              physicalLevelTechnique,
              encodingOption,
              numGeometries,
              numParts,
              numRings,
              numTriangles,
              indexBuffer,
              encodePolygonOutlines);
      result.addAll(encodedVertexBufferStream);
      return new EncodedGeometryColumn(
          numStreams + 1, result, vertexLimits.max, geometryColumnSorted);
    }

    if (appendLengthStream(
        result, numGeometries, physicalLevelTechnique, LengthType.GEOMETRIES, encodingOption)) {
      numStreams++;
    }
    if (appendLengthStream(
        result, numParts, physicalLevelTechnique, LengthType.PARTS, encodingOption)) {
      numStreams++;
    }
    if (appendLengthStream(
        result, numRings, physicalLevelTechnique, LengthType.RINGS, encodingOption)) {
      numStreams++;
    }

    @NotNull final ArrayList<byte[]> selectedVertexStream;
    @Nullable final int[] selectedVertexOffsets;
    if (plainVertexBufferSize <= dictionaryEncodedSize
        && (!useMortonEncoding || plainVertexBufferSize <= mortonDictionaryEncodedSize)) {
      selectedVertexStream = encodedVertexBufferStream;
      selectedVertexOffsets = null;
    } else if (!useMortonEncoding || dictionaryEncodedSize <= mortonDictionaryEncodedSize) {
      selectedVertexOffsets = vertexDictionaryOffsets;
      selectedVertexStream =
          encodeVertexBuffer(zigZagDeltaVertexDictionary, physicalLevelTechnique);
      geometryColumnSorted = false;
    } else {
      selectedVertexOffsets = mortonEncodedDictionaryOffsets;
      selectedVertexStream =
          IntegerEncoder.encodeMortonStream(
              mortonEncodedDictionary,
              zOrderCurve.numBits(),
              zOrderCurve.coordinateShift(),
              physicalLevelTechnique);
    }

    if (selectedVertexOffsets != null && selectedVertexOffsets.length > 0) {
      result.addAll(
          IntegerEncoder.encodeIntStream(
              selectedVertexOffsets,
              physicalLevelTechnique,
              false,
              PhysicalStreamType.OFFSET,
              new LogicalStreamType(OffsetType.VERTEX),
              encodingOption));
      numStreams++;
    }
    result.addAll(selectedVertexStream);
    return new EncodedGeometryColumn(
        numStreams + 1, result, vertexLimits.max, geometryColumnSorted);
  }

  private static record MinMax<T>(T min, T max) {}

  private static MinMax<Integer> getVertexLimits(List<Vertex> vertexBuffer) {
    var minVertexValue = Integer.MAX_VALUE;
    var maxVertexValue = Integer.MIN_VALUE;
    for (final var vertex : vertexBuffer) {
      final var x = vertex.x();
      final var y = vertex.y();
      if (x < minVertexValue) minVertexValue = x;
      if (y < minVertexValue) minVertexValue = y;
      if (x > maxVertexValue) maxVertexValue = x;
      if (y > maxVertexValue) maxVertexValue = y;
    }
    return new MinMax<Integer>(minVertexValue, maxVertexValue);
  }

  /// Non-tessellation overload
  private static void prepareGeometry(
      List<Geometry> geometries,
      ArrayList<Integer> numGeometries,
      ArrayList<Integer> geometryTypes,
      ArrayList<Vertex> vertexBuffer,
      ArrayList<Integer> numParts,
      ArrayList<Integer> numRings) {
    prepareGeometry(
        geometries,
        numGeometries,
        geometryTypes,
        vertexBuffer,
        numParts,
        numRings,
        null,
        null,
        null);
  }

  /// Break geometry down into encodable components
  /// Optional tessellation overload
  private static void prepareGeometry(
      List<Geometry> geometries,
      ArrayList<Integer> numGeometries,
      ArrayList<Integer> geometryTypes,
      ArrayList<Vertex> vertexBuffer,
      ArrayList<Integer> numParts,
      ArrayList<Integer> numRings,
      ArrayList<Integer> numTriangles,
      ArrayList<Integer> indexBuffer,
      @Nullable URI tessellateSource) {
    final var containsPolygon = containsPolygon(geometries);
    final var tessellate = (numTriangles != null && indexBuffer != null);
    for (var geometry : geometries) {
      switch (geometry) {
        case Point point -> {
          geometryTypes.add(GeometryType.POINT.ordinal());
          vertexBuffer.add(new Vertex((int) point.getX(), (int) point.getY()));
        }
        case LineString lineString -> {
          // TODO: verify if part of a MultiPolygon or Polygon geometry add then to numRings?
          geometryTypes.add(GeometryType.LINESTRING.ordinal());
          final var numVertices = lineString.getCoordinates().length;
          addLineString(containsPolygon, numVertices, numParts, numRings);
          vertexBuffer.addAll(flatLineString(lineString));
        }
        case Polygon polygon -> {
          geometryTypes.add(GeometryType.POLYGON.ordinal());
          flatPolygon(polygon, vertexBuffer, numParts, numRings);

          if (tessellate) {
            final var tessellatedPolygon =
                TessellationUtils.tessellatePolygon(polygon, 0, tessellateSource);
            numTriangles.add(tessellatedPolygon.numTriangles());
            indexBuffer.addAll(tessellatedPolygon.indexBuffer());
          }
        }
        case MultiLineString multiLineString -> {
          // TODO: verify if part of a MultiPolygon or Polygon geometry add then to numRings?
          geometryTypes.add(GeometryType.MULTILINESTRING.ordinal());
          final var numLineStrings = multiLineString.getNumGeometries();
          numGeometries.add(numLineStrings);
          for (var i = 0; i < numLineStrings; i++) {
            final var lineString = (LineString) multiLineString.getGeometryN(i);
            final var numVertices = lineString.getCoordinates().length;
            addLineString(containsPolygon, numVertices, numParts, numRings);
            vertexBuffer.addAll(flatLineString(lineString));
          }
        }
        case MultiPolygon multiPolygon -> {
          geometryTypes.add(GeometryType.MULTIPOLYGON.ordinal());
          final var numPolygons = multiPolygon.getNumGeometries();
          numGeometries.add(numPolygons);
          for (var i = 0; i < numPolygons; i++) {
            final var polygon = (Polygon) multiPolygon.getGeometryN(i);
            flatPolygon(polygon, vertexBuffer, numParts, numRings);
          }

          // TODO: use also a vertex dictionary encoding for MultiPolygon geometries
          if (tessellate) {
            final var tessellatedPolygon =
                TessellationUtils.tessellateMultiPolygon(multiPolygon, tessellateSource);
            numTriangles.add(tessellatedPolygon.numTriangles());
            indexBuffer.addAll(tessellatedPolygon.indexBuffer());
          }
        }
        case MultiPoint multiPoint -> {
          geometryTypes.add(GeometryType.MULTIPOINT.ordinal());
          final var numPoints = multiPoint.getNumGeometries();
          numGeometries.add(numPoints);
          for (var i = 0; i < numPoints; i++) {
            final var point = (Point) multiPoint.getGeometryN(i);
            vertexBuffer.add(new Vertex((int) point.getX(), (int) point.getY()));
          }
        }
        default ->
            throw new IllegalArgumentException(
                "Specified geometry type is not (yet) supported: " + geometry.getGeometryType());
      }
    }
  }

  private static int encodePolygonPretessellationStreams(
      final ArrayList<byte[]> result,
      PhysicalLevelTechnique physicalLevelTechnique,
      @NotNull IntegerEncodingOption encodingOption,
      ArrayList<Integer> numGeometries,
      ArrayList<Integer> numParts,
      ArrayList<Integer> numRings,
      ArrayList<Integer> numTriangles,
      ArrayList<Integer> indexBuffer,
      boolean withOutlines)
      throws IOException {
    if (withOutlines) {
      Objects.requireNonNull(numGeometries);
      Objects.requireNonNull(numParts);
      Objects.requireNonNull(numRings);
    }
    int numStreams = 1;

    // TODO: Don't write empty streams
    final boolean forceEmpty = true;

    if (withOutlines) {
      if (appendLengthStream(
          result,
          numGeometries,
          physicalLevelTechnique,
          LengthType.GEOMETRIES,
          encodingOption,
          forceEmpty)) {
        numStreams++;
      }
      if (appendLengthStream(
          result, numParts, physicalLevelTechnique, LengthType.PARTS, encodingOption, forceEmpty)) {
        numStreams++;
      }
      if (appendLengthStream(
          result, numRings, physicalLevelTechnique, LengthType.RINGS, encodingOption, forceEmpty)) {
        numStreams++;
      }
    }
    if (appendLengthStream(
        result, numTriangles, physicalLevelTechnique, LengthType.TRIANGLES, encodingOption)) {
      numStreams++;
    }
    result.addAll(
        IntegerEncoder.encodeIntStream(
            indexBuffer,
            physicalLevelTechnique,
            false,
            PhysicalStreamType.OFFSET,
            new LogicalStreamType(OffsetType.INDEX),
            encodingOption));
    return numStreams;
  }

  private static boolean appendLengthStream(
      @NotNull ArrayList<byte[]> result,
      @Nullable List<Integer> values,
      @NotNull PhysicalLevelTechnique physicalLevelTechnique,
      @NotNull LengthType lengthType,
      @NotNull IntegerEncodingOption encodingOption)
      throws IOException {
    return appendLengthStream(
        result, values, physicalLevelTechnique, lengthType, encodingOption, false);
  }

  private static boolean appendLengthStream(
      @NotNull ArrayList<byte[]> result,
      @Nullable List<Integer> values,
      @NotNull PhysicalLevelTechnique physicalLevelTechnique,
      @NotNull LengthType lengthType,
      @NotNull IntegerEncodingOption encodingOption,
      boolean forceWriteEmptyStream)
      throws IOException {
    if (values != null && !values.isEmpty() || forceWriteEmptyStream) {
      result.addAll(
          IntegerEncoder.encodeIntStream(
              (values != null) ? values : List.of(),
              physicalLevelTechnique,
              false,
              PhysicalStreamType.LENGTH,
              new LogicalStreamType(lengthType),
              encodingOption));
      return true;
    }
    return false;
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

  private static void addLineString(
      boolean containsPolygon, int numVertices, List<Integer> numParts, List<Integer> numRings) {
    /* Depending on the max geometry type in the column add to the numRings or numParts stream */
    if (containsPolygon) {
      numRings.add(numVertices);
    } else {
      numParts.add(numVertices);
    }
  }

  public static int[] zigZagDeltaEncodeVertices(@NotNull final Collection<Vertex> vertices) {
    return zigZagDeltaEncodeVertices(vertices.stream(), vertices.size());
  }

  public static int[] zigZagDeltaEncodeVertices(@NotNull final Vertex[] vertices) {
    return zigZagDeltaEncodeVertices(
        StreamSupport.stream(Arrays.spliterator(vertices), false), vertices.length);
  }

  private static int[] zigZagDeltaEncodeVertices(
      @NotNull final Stream<Vertex> vertices, final int size) {
    int prevX = 0;
    int prevY = 0;
    int j = 0;
    final var deltaValues = new int[size * 2];
    for (var iter = vertices.iterator(); iter.hasNext(); ) {
      final var vertex = iter.next();
      final var x = vertex.x();
      final var y = vertex.y();
      deltaValues[j++] = EncodingUtils.encodeZigZag(x - prevX);
      deltaValues[j++] = EncodingUtils.encodeZigZag(y - prevY);
      prevX = x;
      prevY = y;
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

  private static Map<Integer, Integer> reverseMap(IntStream mortonEncodedDictionary, int size) {
    Map<Integer, Integer> morton = HashMap.newHashMap(size);
    int i = 0;
    for (var iter = mortonEncodedDictionary.iterator(); iter.hasNext(); ) {
      morton.put(iter.nextInt(), i++);
    }
    return morton;
  }

  private static Map<Integer, Integer> reverseMap(Collection<Integer> mortonEncodedDictionary) {
    return reverseMap(
        mortonEncodedDictionary.stream().mapToInt(Integer::intValue),
        mortonEncodedDictionary.size());
  }

  private static Map<Integer, Integer> reverseMap(int[] mortonEncodedDictionary) {
    return reverseMap(IntStream.of(mortonEncodedDictionary), mortonEncodedDictionary.length);
  }

  /// An entry in the vertex dictionary, used for sorting and filtering duplicates
  record Indexed(int hilbert, int index) implements Comparable<Indexed> {
    @Override
    public int compareTo(@NotNull GeometryEncoder.Indexed o) {
      return Integer.compare(hilbert, o.hilbert);
    }
  }

  /// A predicate for filtering consecutive duplicates from streams of `Indexed`
  private static Predicate<Indexed> distinctByHilbertId() {
    return new Predicate<Indexed>() {
      private boolean first = true;
      private int lastSeen;

      @Override
      public boolean test(Indexed indexed) {
        if (first || indexed.hilbert != lastSeen) {
          lastSeen = indexed.hilbert;
          first = false;
          return true;
        }
        return false;
      }
    };
  }

  private static Pair<int[], Vertex[]> addVerticesToDictionary(
      @NotNull final ArrayList<Vertex> vertices, @NotNull final HilbertCurve hilbertCurve) {
    // 1. Convert to (hilbertId, vertex) pairs
    // 2. Sort by hilbertId
    // 3. Filter consecutive duplicates
    // 4. Convert back to separate arrays for hilbertIds and vertices
    // Can we do this without materializing the intermediate list?
    final var vertexDictionary =
        IntStream.range(0, vertices.size())
            .mapToObj(i -> new Indexed(hilbertCurve.encode(vertices.get(i)), i))
            .sorted(Comparator.naturalOrder())
            // TODO: we currently don't filter duplicates in the vertex dictionary!
            // .filter(distinctByHilbertId())
            .toList();
    return Pair.of(
        vertexDictionary.stream().mapToInt(Indexed::hilbert).toArray(),
        vertexDictionary.stream().map(i -> vertices.get(i.index)).toArray(Vertex[]::new));
  }

  private static int[] addVerticesToMortonDictionary(
      @NotNull final Collection<Vertex> vertices, @NotNull final ZOrderCurve zOrderCurve) {
    return vertices.stream().mapToInt(zOrderCurve::encode).sorted().toArray();
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
  private static ArrayList<byte[]> encodeVertexBuffer(
      int[] values, PhysicalLevelTechnique physicalLevelTechnique) throws IOException {
    final var encodedValues =
        physicalLevelTechnique == PhysicalLevelTechnique.FAST_PFOR
            ? encodeFastPfor(values, false)
            : encodeVarint(values, false);

    final var result =
        new StreamMetadata(
                PhysicalStreamType.DATA,
                new LogicalStreamType(DictionaryType.VERTEX),
                LogicalLevelTechnique.COMPONENTWISE_DELTA,
                LogicalLevelTechnique.NONE,
                physicalLevelTechnique,
                values.length,
                encodedValues.length)
            .encode();

    result.add(encodedValues);
    return result;
  }
}
