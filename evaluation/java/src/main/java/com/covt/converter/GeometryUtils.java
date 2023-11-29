package com.covt.converter;

import com.covt.converter.geometry.Vertex;
import org.davidmoten.hilbert.SmallHilbertCurve;

public class GeometryUtils {

    public static int getHilbertIndex(SmallHilbertCurve curve, Vertex vertex, int tileExtent){
        var shiftedX = tileExtent + vertex.x();
        var shiftedY = tileExtent + vertex.y();
        return (int) curve.index(shiftedX, shiftedY);
    }

    public static long[] decodeHilbertIndex(SmallHilbertCurve curve, long hilbertIndex, int tileExtent){
        var point = curve.point(hilbertIndex);
        var x = point[0] - tileExtent;
        var y = point[1] - tileExtent;
        return new long[]{x,y};
    }

    public static int encodeMorton(int x, int y, int numBits){
        int mortonCode = 0;
        for (int i = 0; i < numBits; i++) {
            mortonCode |= (x & (1 << i)) << i | (y & (1 << i)) << (i + 1);
        }
        return mortonCode;
    }

    public static int[] decodeMorton(int mortonCode, int numBtis) {
        int x = decodeMortonCode(mortonCode, numBtis);
        int y = decodeMortonCode(mortonCode >> 1, numBtis);
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
