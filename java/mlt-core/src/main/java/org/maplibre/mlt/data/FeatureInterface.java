package org.maplibre.mlt.data;

import jakarta.annotation.Nullable;
import java.util.Optional;
import java.util.function.Predicate;
import java.util.stream.Stream;
import org.jetbrains.annotations.NotNull;
import org.locationtech.jts.geom.Geometry;
import org.maplibre.mlt.metadata.tileset.MltMetadata;

public interface FeatureInterface {
  boolean hasId();

  long getId();

  @NotNull
  Geometry getGeometry();

  Iterable<Property> getProperties();

  Stream<Property> getPropertyStream();

  Stream<Property> getPropertyStream(boolean parallel);

  Optional<Property> findProperty(@NotNull Predicate<Property> predicate);

  Optional<Property> findProperty(@NotNull String name);

  Optional<Property> findProperty(@NotNull String name, @NotNull MltMetadata.ScalarType type);

  /**
   * Returns the ID as a boxed Long, or null if the feature has no ID. Use this where a nullable
   * Long is required, e.g., intermediate lists, serialization. Prefer {@link #hasId()} and {@link
   * #getId()} in hot paths.
   */
  @Nullable
  Long idOrNull();
}
