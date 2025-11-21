package org.maplibre.mlt.converter;

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
}
