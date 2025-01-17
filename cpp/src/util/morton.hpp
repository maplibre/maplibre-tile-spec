#pragma once

#include <common.hpp>

namespace mlt::util::decoding {

#if 0
  public static int[] decodeMortonCode(List<Integer> mortonCodes, ZOrderCurve zOrderCurve) {
    var vertexBuffer = new int[mortonCodes.size() * 2];
    for (var i = 0; i < mortonCodes.size(); i++) {
      var mortonCode = mortonCodes.get(i);
      var vertex = zOrderCurve.decode(mortonCode);
      vertexBuffer[i * 2] = vertex[0];
      vertexBuffer[i * 2 + 1] = vertex[1];
    }

    return vertexBuffer;
  }
#endif

}
