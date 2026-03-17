package org.maplibre.mlt.data;

import jakarta.annotation.Nullable;
import java.math.BigInteger;
import java.util.Collections;
import java.util.Map;
import java.util.Objects;
import java.util.Optional;
import java.util.stream.Stream;
import org.jetbrains.annotations.NotNull;
import org.locationtech.jts.geom.Geometry;
import org.maplibre.mlt.data.unsigned.U32;
import org.maplibre.mlt.data.unsigned.U64;
import org.maplibre.mlt.data.unsigned.U8;
import org.maplibre.mlt.metadata.tileset.MltMetadata;

public class MVTFeature implements Feature {
  private final boolean hasId;
  private final long id;
  private final Geometry geometry;
  private final Map<String, Object> properties;

  private MVTFeature(
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

  public Map<String, Object> getRawProperties() {
    return properties;
  }

  @Override
  public int getPropertyCount() {
    return properties.size();
  }

  @Override
  public Stream<String> getPropertyKeys() {
    return properties.entrySet().stream().map(Map.Entry::getKey);
  }

  @Override
  public Optional<Property> findProperty(String name) {
    return Optional.ofNullable(properties.get(name)).map(value -> adapt(name, value));
  }

  @Override
  public Stream<Property> getPropertyStream(boolean parallel) {
    final var entries = properties.entrySet();
    return (parallel ? entries.parallelStream() : entries.stream()).map(MVTFeature::adapt);
  }

  static Property adapt(String name, Object value) {
    final var type = getType(value);
    if (type.scalarType == null) {
      // MVT only supports scalar types
      throw new IllegalArgumentException(
          "Unsupported property value type: " + value.getClass() + " for property: " + name);
    }
    return new Property(type.scalarType.physicalType, name, value);
  }

  static Property adapt(Map.Entry<String, Object> entry) {
    return adapt(entry.getKey(), entry.getValue());
  }

  static MltMetadata.FieldType getType(Object value) {
    return switch (value) {
      case null -> makeField(MltMetadata.ScalarType.UNRECOGNIZED);
      case String s -> makeField(MltMetadata.ScalarType.STRING);
      case Boolean b -> makeField(MltMetadata.ScalarType.BOOLEAN);
      case Double d -> makeField(MltMetadata.ScalarType.DOUBLE);
      case Float f -> makeField(MltMetadata.ScalarType.FLOAT);
      case U8 i -> makeField(MltMetadata.ScalarType.UINT_8);
      case Integer i -> makeField(MltMetadata.ScalarType.INT_32);
      case U32 i -> makeField(MltMetadata.ScalarType.UINT_32);
      case Long l -> makeField(MltMetadata.ScalarType.INT_64);
      case U64 i -> makeField(MltMetadata.ScalarType.UINT_64);
      case BigInteger i -> makeField(MltMetadata.ScalarType.UINT_64);
      default ->
          throw new IllegalArgumentException(
              "Unsupported property value type: " + value.getClass());
    };
  }

  private static MltMetadata.FieldType makeField(MltMetadata.ScalarType type) {
    final var isNullable = true;
    return new MltMetadata.FieldType(new MltMetadata.ScalarField(type), isNullable);
  }

  @Override
  public boolean equals(Object o) {
    if (this == o) return true;
    if (o == null || getClass() != o.getClass()) return false;
    final var feature = (MVTFeature) o;
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
    return "MVTFeature[hasId="
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

  public static Builder builder(@Nullable MVTFeature feature) {
    return (feature == null)
        ? new Builder()
        : new Builder()
            .id(feature.idOrNull())
            .geometry(feature.geometry)
            .properties(feature.properties);
  }

  public static class Builder extends Feature.AbstractBuilder<Builder, MVTFeature> {
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
    public MVTFeature build() {
      if (geometry == null) {
        throw new IllegalStateException("geometry is required");
      }
      return new MVTFeature(
          hasId, id, geometry, (properties == null) ? Collections.emptyMap() : properties);
    }
  }

  @Override
  @SuppressWarnings("unchecked")
  public <B extends Feature.Builder<B, F>, F extends Feature> Feature.Builder<B, F> asBuilder() {
    return (Feature.AbstractBuilder<B, F>) builder(this);
  }
}
