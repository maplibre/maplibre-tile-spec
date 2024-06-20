package com.mlt.converter.geometry;

import com.google.common.collect.Lists;
import java.util.*;
import org.apache.commons.lang3.tuple.Triple;

public class GeometryUtils {

  private GeometryUtils() {}

  public static void sortVertexOffsets(
      List<Integer> numParts, List<Integer> mortonEncodedDictionaryOffsets, List<Long> featureIds) {
    // TODO: use an different proper optimization approach
    /*
     * Quick and dirty approach to sort the VertexOffsets of a VertexBuffer to reduce the deltas
     * and therefore the overall size.
     * The offsets are sorted based on the morton code of the first  vertex of a LineString.
     * The order of the offsets of a LineString has to be preserved.
     * */
    var sortedDictionaryOffsets =
        new TreeMap<Integer, Triple<List<Long>, List<Integer>, List<Integer>>>();
    var partOffsetCounter = 0;
    var idCounter = 0;
    for (var numPart : numParts) {
      var currentLineVertexOffsets =
          mortonEncodedDictionaryOffsets.subList(partOffsetCounter, partOffsetCounter + numPart);
      partOffsetCounter += numPart;

      var featureId = featureIds.get(idCounter++);
      var minVertexOffset = currentLineVertexOffsets.get(0);
      if (sortedDictionaryOffsets.containsKey(minVertexOffset)) {
        var sortedDictionaryOffset = sortedDictionaryOffsets.get(minVertexOffset);
        sortedDictionaryOffset.getLeft().add(featureId);
        sortedDictionaryOffset.getMiddle().addAll(currentLineVertexOffsets);
        sortedDictionaryOffset.getRight().add(numPart);
      } else {
        sortedDictionaryOffsets.put(
            minVertexOffset,
            Triple.of(
                Lists.newArrayList(featureId),
                new ArrayList<>(currentLineVertexOffsets),
                Lists.newArrayList(numPart)));
      }
    }

    var sortedOffsets =
        sortedDictionaryOffsets.values().stream().flatMap(e -> e.getMiddle().stream()).toList();
    var updatedFeatureIds =
        sortedDictionaryOffsets.values().stream().flatMap(e -> e.getLeft().stream()).toList();
    var updatedNumParts =
        sortedDictionaryOffsets.values().stream().flatMap(e -> e.getRight().stream()).toList();

    mortonEncodedDictionaryOffsets.clear();
    mortonEncodedDictionaryOffsets.addAll(sortedOffsets);
    featureIds.clear();
    featureIds.addAll(updatedFeatureIds);
    numParts.clear();
    numParts.addAll(updatedNumParts);
  }

  public static void sortPoints(
      List<Vertex> points, HilbertCurve hilbertCurve, List<Long> featureIds) {
    var sortedPoints = new ArrayList<Triple<Integer, Long, Vertex>>();
    for (var i = 0; i < points.size(); i++) {
      var featureId = featureIds.get(i);
      var point = points.get(i);
      var hilbertId = hilbertCurve.encode(point);
      sortedPoints.add(Triple.of(hilbertId, featureId, point));
    }

    sortedPoints.sort(Comparator.comparingInt(Triple::getLeft));
    featureIds.clear();
    featureIds.addAll(sortedPoints.stream().map(Triple::getMiddle).toList());
    points.clear();
    points.addAll(sortedPoints.stream().map(Triple::getRight).toList());
  }
}
