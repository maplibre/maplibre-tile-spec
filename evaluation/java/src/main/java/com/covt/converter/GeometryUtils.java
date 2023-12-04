package com.covt.converter;

import com.covt.converter.geometry.Vertex;
import org.davidmoten.hilbert.SmallHilbertCurve;

public class GeometryUtils {

    public static int encodeHilbertIndex(SmallHilbertCurve curve, Vertex vertex){
        var tileExtent = 2 << (curve.bits() - 2);
        var shiftedX = tileExtent/2 + vertex.x();
        var shiftedY = tileExtent/2 + vertex.y();
        return (int) curve.index(shiftedX, shiftedY);
    }

    public static long[] decodeHilbertIndex(SmallHilbertCurve curve, long hilbertIndex){
        var tileExtent = 2 << (curve.bits() - 2);
        var point = curve.point(hilbertIndex);
        var x = point[0] - tileExtent/2;
        var y = point[1] - tileExtent/2;
        return new long[]{x,y};
    }

    public static int encodeMorton(int x, int y, int numBits){
        var tileExtent = 2 << (numBits - 2);
        x = x + tileExtent/2;
        y = y + tileExtent/2;
        int mortonCode = 0;
        for (int i = 0; i < numBits; i++) {
            mortonCode |= (x & (1 << i)) << i | (y & (1 << i)) << (i + 1);
        }
        return mortonCode;
    }

    public static int[] decodeMorton(int mortonCode, int numBits) {
        var tileExtent = 2 << (numBits - 2);
        int x = decodeMortonCode(mortonCode, numBits) - tileExtent/2;
        int y = decodeMortonCode(mortonCode >> 1, numBits) - tileExtent/2;
        return new int[]{x, y};
    }

    private static int decodeMortonCode (int code, int numBits) {
        int coordinate = 0;
        for (int i = 0; i < numBits; i++) {
            coordinate |= (code & (1L << (2 * i))) >> i;
        }
        return coordinate;
    }
}
