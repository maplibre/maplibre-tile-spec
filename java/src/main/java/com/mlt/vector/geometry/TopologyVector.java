package com.mlt.vector.geometry;

import java.nio.IntBuffer;

public record TopologyVector(IntBuffer geometryOffsets, IntBuffer partOffsets, IntBuffer ringOffsets){ }
