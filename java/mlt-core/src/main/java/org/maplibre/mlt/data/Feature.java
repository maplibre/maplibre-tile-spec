package org.maplibre.mlt.data;

import jakarta.annotation.Nullable;
import java.util.Optional;
import java.util.function.Predicate;
import java.util.stream.Stream;
import lombok.Builder;
import lombok.EqualsAndHashCode;
import lombok.Getter;
import lombok.experimental.Accessors;
import lombok.experimental.SuperBuilder;
import org.jetbrains.annotations.NotNull;
import org.locationtech.jts.geom.Geometry;
import org.maplibre.mlt.metadata.tileset.MltMetadata;

@SuperBuilder(toBuilder = true)
@EqualsAndHashCode
public abstract class Feature implements FeatureInterface {
  @Getter protected final int index;

  @Accessors(fluent = true)
  @Getter
  @Builder.Default
  protected final boolean hasId = false;

  @Getter @Builder.Default long id = 0;

  @Getter @NotNull protected final Geometry geometry;

  public Iterable<Property> getProperties() {
    return () -> getPropertyStream().iterator();
  }

  public Stream<Property> getPropertyStream() {
    return getPropertyStream(false);
  }

  public Optional<Property> findProperty(@NotNull Predicate<Property> predicate) {
    return getPropertyStream().filter(predicate).findFirst();
  }

  public Optional<Property> findProperty(@NotNull String name) {
    return findProperty(p -> p.getName().equals(name));
  }

  public Optional<Property> findProperty(
      @NotNull String name, @NotNull MltMetadata.ScalarType type) {
    return findProperty(name)
        .filter(
            p ->
                !p.isNested()
                    && p.getType().scalarType != null
                    && p.getType().scalarType.physicalType.equals(type));
  }

  /**
   * Returns the ID as a boxed Long, or null if the feature has no ID. Use this where a nullable
   * Long is required, e.g., intermediate lists, serialization. Prefer {@link #hasId()} and {@link
   * #getId()} in hot paths.
   */
  @Nullable
  public Long idOrNull() {
    return hasId() ? getId() : null;
  }

  public abstract static class FeatureBuilder<C extends Feature, B extends FeatureBuilder<C, B>> {
    public B id(long id) {
      this.id$value = id;
      this.id$set = true;
      return hasId(true);
    }

    public B id(@Nullable Long id) {
      return (id != null) ? id(id.longValue()) : hasId(false);
    }
  }

  public abstract FeatureBuilder<?, ?> toBuilder();
}
