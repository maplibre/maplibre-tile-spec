package org.maplibre.mlt.data;

import jakarta.annotation.Nullable;
import java.util.Collections;
import java.util.Map;
import java.util.Objects;
import java.util.Optional;
import java.util.stream.Stream;
import org.jetbrains.annotations.NotNull;
import org.locationtech.jts.geom.Geometry;
import org.maplibre.mlt.metadata.tileset.MltMetadata;

public class MLTFeature implements Feature {
  private final boolean hasId;
  private final long id;
  private final Geometry geometry;
  private final Map<String, Object> properties;

  private MLTFeature(
      boolean hasId, long id, @NotNull Geometry geometry, @NotNull Map<String, Object> properties) {
    this.hasId = hasId;
    this.id = id;
    this.geometry = geometry;
    this.properties = properties;
  }

  @Override
  public boolean hasId() {
    return hasId;
  }

  @Override
  public long getId() {
    return id;
  }

  @Override
  public Geometry getGeometry() {
    return geometry;
  }

  @Override
  public Optional<Property> findProperty(String name) {
    return Optional.ofNullable(properties.get(name)).map(value -> adapt(name, value));
  }

  @Override
  public Stream<Property> getPropertyStream(boolean parallel) {
    final var entries = properties.entrySet();
    return (parallel ? entries.parallelStream() : entries.stream()).map(MLTFeature::adapt);
  }

  static Property adapt(String name, Object value) {
    return new Property(getType(value), name, value);
  }

  static Property adapt(Map.Entry<String, Object> entry) {
    return adapt(entry.getKey(), entry.getValue());
  }

  static MltMetadata.FieldType getType(Object value) {
    if (value instanceof Map<?, ?>) {
      final boolean isNullable = true;
      return new MltMetadata.FieldType(
          new MltMetadata.ComplexField(MltMetadata.ComplexType.MAP), isNullable);
    }
    return MVTFeature.getType(value);
  }

  @Override
  public boolean equals(Object o) {
    if (this == o) return true;
    if (o == null || getClass() != o.getClass()) return false;
    final var feature = (MLTFeature) o;
    return hasId == feature.hasId
        && id == feature.id
        && Objects.equals(geometry, feature.geometry)
        && Objects.equals(properties, feature.properties);
  }

  @Override
  public int hashCode() {
    return Objects.hash(hasId, id, geometry, properties);
  }

  @Override
  public String toString() {
    return "MLTFeature[hasId="
        + hasId
        + ", id="
        + id
        + ", geometry="
        + geometry
        + ", properties="
        + properties
        + "]";
  }

  public static Builder builder() {
    return new Builder();
  }

  public static Builder builder(@Nullable MLTFeature feature) {
    return (feature == null)
        ? new Builder()
        : new Builder()
            .id(feature.idOrNull())
            .geometry(feature.geometry)
            .properties(feature.properties);
  }

  public static class Builder extends Feature.AbstractBuilder<Builder, MLTFeature> {
    @Nullable private Map<String, Object> properties = null;

    @Override
    public Builder id(long id) {
      this.id = id;
      this.hasId = true;
      return this;
    }

    @Override
    public Builder id(@Nullable Long id) {
      this.hasId = (id != null);
      this.id = hasId ? id : 0;
      return this;
    }

    @Override
    public Builder geometry(@Nullable Geometry geometry) {
      this.geometry = geometry;
      return this;
    }

    @Override
    public Builder properties(@Nullable Map<String, Object> properties) {
      this.properties = properties;
      return this;
    }

    @Override
    public MLTFeature build() {
      if (geometry == null) {
        throw new IllegalStateException("geometry is required");
      }
      return new MLTFeature(
          hasId, id, geometry, (properties == null) ? Collections.emptyMap() : properties);
    }
  }

  @Override
  @SuppressWarnings("unchecked")
  public <B extends Feature.Builder<B, F>, F extends Feature> Feature.Builder<B, F> asBuilder() {
    return (Feature.AbstractBuilder<B, F>) builder(this);
  }
}
