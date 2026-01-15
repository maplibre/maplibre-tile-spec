package org.maplibre.mlt.converter;

import java.util.Collection;
import org.apache.commons.lang3.ArrayUtils;

public class CollectionUtils {
  private CollectionUtils() {}

  public static byte[] concatByteArrays(byte[]... arrays) {
    var concatenatedArray = new byte[0];
    for (var array : arrays) {
      concatenatedArray = ArrayUtils.addAll(concatenatedArray, array);
    }
    return concatenatedArray;
  }

  public static int[] unboxInts(Collection<? extends Number> values) {
    int i = 0;
    int[] result = new int[values.size()];
    for (var value : values) {
      result[i++] = value.intValue();
    }
    return result;
  }

  public static long[] unboxLongs(Collection<? extends Number> values) {
    int i = 0;
    long[] result = new long[values.size()];
    for (var value : values) {
      result[i++] = value.longValue();
    }
    return result;
  }
}
