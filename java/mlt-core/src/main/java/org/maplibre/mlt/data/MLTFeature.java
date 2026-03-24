package org.maplibre.mlt.data;

import java.util.Map;
import java.util.Optional;
import java.util.stream.Stream;
import lombok.Builder;
import lombok.EqualsAndHashCode;
import lombok.experimental.SuperBuilder;
import org.jetbrains.annotations.NotNull;
import org.maplibre.mlt.metadata.tileset.MltMetadata;

@SuperBuilder(toBuilder = true)
@EqualsAndHashCode(callSuper = true)
public class MLTFeature extends Feature {
  @NotNull @Builder.Default private final Map<String, Object> properties = Map.of();

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
      return MltMetadata.Field.builder()
          .complexType(new MltMetadata.ComplexField(MltMetadata.ComplexType.MAP))
          .isNullable(isNullable)
          .build();
    }
    return MVTFeature.getType(value);
  }
}
