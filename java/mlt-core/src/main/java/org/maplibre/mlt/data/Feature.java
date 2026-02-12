package org.maplibre.mlt.data;

import jakarta.annotation.Nullable;
import java.util.Map;
import org.locationtech.jts.geom.Geometry;

public record Feature(@Nullable Long id, Geometry geometry, Map<String, Object> properties) {}
