package org.maplibre.mlt.data;

import jakarta.annotation.Nullable;
import java.util.Map;
import org.locationtech.jts.geom.Geometry;

public record Feature(boolean hasId, long id, Geometry geometry, Map<String, Object> properties) {

  /** Creates a feature with an ID. */
  public Feature(long id, Geometry geometry, Map<String, Object> properties) {
    this(true, id, geometry, properties);
  }

  /** Creates a feature without an ID. */
  public Feature(Geometry geometry, Map<String, Object> properties) {
    this(false, 0, geometry, properties);
  }

  /**
   * Returns the ID as a boxed Long, or null if the feature has no ID. Use this only where a
   * nullable Long is required (e.g., intermediate lists, serialization). Prefer {@link #hasId()} +
   * {@link #id()} in hot paths.
   */
  @Nullable
  public Long idOrNull() {
    return hasId ? id : null;
  }
}
