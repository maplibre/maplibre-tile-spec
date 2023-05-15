package com.covt.evaluation;

import org.locationtech.jts.geom.Geometry;

import java.util.Map;

public record HilbertFeature (long id, Geometry geometry, Map<String, Object> properties,
                              int hilbertId){}