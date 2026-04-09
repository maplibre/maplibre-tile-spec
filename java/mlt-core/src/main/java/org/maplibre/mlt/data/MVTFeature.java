package org.maplibre.mlt.data;

import java.math.BigInteger;
import java.util.Map;
import java.util.Optional;
import java.util.stream.Stream;
import lombok.Builder;
import lombok.EqualsAndHashCode;
import lombok.experimental.SuperBuilder;
import org.jetbrains.annotations.NotNull;
import org.maplibre.mlt.data.unsigned.U32;
import org.maplibre.mlt.data.unsigned.U64;
import org.maplibre.mlt.data.unsigned.U8;
import org.maplibre.mlt.metadata.tileset.MltMetadata;

@SuperBuilder(toBuilder = true)
@EqualsAndHashCode(callSuper = true)
public class MVTFeature extends Feature {
  @NotNull @Builder.Default private final Map<String, Object> properties = Map.of();

  public Map<String, Object> getRawProperties() {
    return properties;
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
    final var isNullable = true;
    return new Property(MltMetadata.scalarFieldType(type, isNullable), name, value);
  }

  static Property adapt(Map.Entry<String, Object> entry) {
    return adapt(entry.getKey(), entry.getValue());
  }

  static MltMetadata.ScalarType getType(Object value) {
    return switch (value) {
      case null -> MltMetadata.ScalarType.UNRECOGNIZED;
      case String ignored -> MltMetadata.ScalarType.STRING;
      case Boolean ignored -> MltMetadata.ScalarType.BOOLEAN;
      case Double ignored -> MltMetadata.ScalarType.DOUBLE;
      case Float ignored -> MltMetadata.ScalarType.FLOAT;
      case U8 ignored -> MltMetadata.ScalarType.UINT_8;
      case Integer ignored -> MltMetadata.ScalarType.INT_32;
      case U32 ignored -> MltMetadata.ScalarType.UINT_32;
      case Long l ->
          (l.intValue() == l) ? MltMetadata.ScalarType.INT_32 : MltMetadata.ScalarType.INT_64;
      case U64 ignored -> MltMetadata.ScalarType.UINT_64;
      case BigInteger i ->
          (i.intValue() == i.longValue())
              ? MltMetadata.ScalarType.INT_32
              : MltMetadata.ScalarType.INT_64;
      default ->
          throw new IllegalArgumentException(
              "Unsupported property value type: " + value.getClass());
    };
  }
}
