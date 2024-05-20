package com.mlt.vector.geometry;

import org.locationtech.jts.geom.Geometry;

import java.nio.IntBuffer;
import java.util.Iterator;

public class GeometryVector implements Iterable<Geometry> {

    enum VertexBufferType {
        MORTON,
        VEC_2,
        VEC_3
    }

    private final VertexBufferType vertexBufferType;
    private IntBuffer geometryTypes;
    private int geometryType;
    private final int numGeometries;
    private final TopologyVector topologyVector;
    private final IntBuffer vertexOffsets;
    private final IntBuffer vertexBuffer;

    public GeometryVector(VertexBufferType vertexBufferType, IntBuffer geometryTypes, TopologyVector topologyVector,
                          IntBuffer vertexOffsets, IntBuffer vertexBuffer){
        this.vertexBufferType = vertexBufferType;
        this.geometryTypes = geometryTypes;
        this.topologyVector = topologyVector;
        this.vertexOffsets = vertexOffsets;
        this.vertexBuffer = vertexBuffer;
        this.numGeometries = geometryTypes.capacity();
    }

    public GeometryVector(int numGeometries, int geometryType, VertexBufferType vertexBufferType,
                          TopologyVector topologyVector, IntBuffer vertexOffsets, IntBuffer vertexBuffer){
        this.vertexBufferType = vertexBufferType;
        this.topologyVector = topologyVector;
        this.vertexOffsets = vertexOffsets;
        this.vertexBuffer = vertexBuffer;
        this.numGeometries = numGeometries;
        this.geometryType = geometryType;
    }

    public static GeometryVector createMortonEncodedGeometryVector(IntBuffer geometryTypes, TopologyVector topologyVector,
                                                                   IntBuffer vertexOffsets, IntBuffer vertexBuffer){
        return new GeometryVector(VertexBufferType.MORTON, geometryTypes, topologyVector, vertexOffsets, vertexBuffer);
    }

    public static GeometryVector createConstMortonEncodedGeometryVector(int numGeometries, int geometryType,
                                                                        TopologyVector topologyVector, IntBuffer vertexOffsets,
                                                                        IntBuffer vertexBuffer){
        return new GeometryVector(numGeometries, geometryType, VertexBufferType.MORTON, topologyVector, vertexOffsets,
                vertexBuffer);
    }

    public static GeometryVector create2DGeometryVector(IntBuffer geometryTypes, TopologyVector topologyVector,
                                                 IntBuffer vertexOffsets, IntBuffer vertexBuffer){
        return new GeometryVector(VertexBufferType.VEC_2, geometryTypes, topologyVector, vertexOffsets, vertexBuffer);
    }

    public static GeometryVector createConst2DGeometryVector(int numGeometries, int geometryType, TopologyVector topologyVector,
                                                        IntBuffer vertexOffsets, IntBuffer vertexBuffer){
        return new GeometryVector(numGeometries, geometryType, VertexBufferType.VEC_2, topologyVector, vertexOffsets,
                vertexBuffer);
    }

    @Override
    public Iterator<Geometry> iterator() {
        return new Iterator<>() {
            private int index = 0;

            @Override
            public boolean hasNext() {
                return index < numGeometries;
            }

            @Override
            public Geometry next() {
                var geometryType = geometryTypes.get(index);
                switch (geometryType){
                }
                return null;
            }
        };
    }
}




