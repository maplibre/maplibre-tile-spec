package com.mlt.converter.geometry;

import org.davidmoten.hilbert.SmallHilbertCurve;

public class HilbertCurve extends SpaceFillingCurve {
    private SmallHilbertCurve curve;

    public HilbertCurve(int minVertexValue, int maxVertexValue){
        super(minVertexValue, maxVertexValue);
        this.curve = org.davidmoten.hilbert.HilbertCurve.small().bits(numBits).dimensions(2);
    }

    public int encode(Vertex vertex){
        //validateCoordinates(vertex);
        var shiftedX = vertex.x() + coordinateShift;
        var shiftedY = vertex.y() + coordinateShift;
        return (int) curve.index(shiftedX, shiftedY);
    }

    public int[] decode(int hilbertIndex){
        var point = curve.point(hilbertIndex);
        var x = (int)point[0] - coordinateShift;
        var y = (int)point[1] - coordinateShift;
        return new int[]{x,y};
    }
}
