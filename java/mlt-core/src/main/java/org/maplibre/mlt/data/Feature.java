package org.maplibre.mlt.data;

import jakarta.annotation.Nullable;
import java.util.Map;
import java.util.Optional;
import java.util.function.Predicate;
import java.util.stream.Stream;
import org.jetbrains.annotations.NotNull;
import org.locationtech.jts.geom.Geometry;
import org.maplibre.mlt.metadata.tileset.MltMetadata;

public interface Feature {
  boolean hasId();

  long getId();

  @NotNull
  Geometry getGeometry();

  default Iterable<Property> getProperties() {
    return () -> getPropertyStream().iterator();
  }

  default Stream<Property> getPropertyStream() {
    return getPropertyStream(false);
  }

  Stream<Property> getPropertyStream(boolean parallel);

  default Optional<Property> findProperty(@NotNull Predicate<Property> predicate) {
    return getPropertyStream().filter(predicate).findFirst();
  }

  default Optional<Property> findProperty(@NotNull String name) {
    return findProperty(p -> p.getName().equals(name));
  }

  default Optional<Property> findProperty(
      @NotNull String name, @NotNull MltMetadata.ScalarType type) {
    return findProperty(name).filter(p -> !p.isNestedProperty() && p.getType().equals(type));
  }

  /**
   * Returns the ID as a boxed Long, or null if the feature has no ID. Use this where a nullable
   * Long is required, e.g., intermediate lists, serialization. Prefer {@link #hasId()} and {@link
   * #getId()} in hot paths.
   */
  @Nullable
  default Long idOrNull() {
    return hasId() ? getId() : null;
  }

  @NotNull
  <B extends Builder<B, F>, F extends Feature> Builder<B, F> asBuilder();

  interface Builder<B extends Builder<B, F>, F extends Feature> {
    B id(long id);

    B id(@Nullable Long id);

    B geometry(@Nullable Geometry geometry);

    B properties(@Nullable Map<String, Object> properties);

    F build();
  }

  abstract static class AbstractBuilder<B extends Builder<B, F>, F extends Feature>
      implements Builder<B, F> {
    protected boolean hasId = false;
    protected long id = 0;
    @Nullable protected Geometry geometry = null;
  }
}
