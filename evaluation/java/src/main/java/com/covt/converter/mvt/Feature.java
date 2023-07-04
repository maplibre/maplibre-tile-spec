package com.covt.converter.mvt;

import org.locationtech.jts.geom.Geometry;

import java.util.Map;

public record Feature(long id, Geometry geometry, Map<String, Object> properties){}
