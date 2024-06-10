package com.mlt.converter.mvt;

import java.util.Map;
import org.locationtech.jts.geom.Geometry;

public record Feature(long id, Geometry geometry, Map<String, Object> properties) {}
