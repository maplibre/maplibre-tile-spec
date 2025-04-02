package com.mlt.converter.tessellation;

import java.util.List;

public record TessellatedPolygon(List<Integer> indexBuffer, int numTriangles, int numVertices) {}
