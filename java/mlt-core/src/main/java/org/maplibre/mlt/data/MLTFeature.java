package org.maplibre.mlt.data;

import java.util.Map;
import java.util.Optional;
import java.util.stream.Collectors;
import java.util.stream.Stream;
import lombok.Builder;
import lombok.EqualsAndHashCode;
import lombok.experimental.SuperBuilder;
import org.jetbrains.annotations.NotNull;
import org.maplibre.mlt.metadata.tileset.MltMetadata;

/** An MLT Feature, which may have nested properties that are not supported in MVT. */
@SuperBuilder(toBuilder = true)
@EqualsAndHashCode(callSuper = true)
public class MLTFeature extends Feature {
  @NotNull @Builder.Default private final Map<String, Property> properties = Map.of();

  @Override
  public Optional<Property> findProperty(String name) {
    return Optional.ofNullable(properties.get(name));
  }

  @Override
  public Stream<Property> getPropertyStream(boolean parallel) {
    final var entries = properties.entrySet();
    return (parallel ? entries.parallelStream() : entries.stream()).map(Map.Entry::getValue);
  }

  static Property adapt(String name, Object value) {
    return new Property(getType(value), name, value);
  }

  static Property adapt(Map.Entry<String, Object> entry) {
    return adapt(entry.getKey(), entry.getValue());
  }

  static MltMetadata.FieldType getType(Object value) {
    if (value instanceof Map<?, ?>) {
      return MltMetadata.complexFieldType(MltMetadata.ComplexType.MAP, true);
    }
    final var isNullable = true;
    return MltMetadata.scalarFieldType(MVTFeature.getType(value), isNullable);
  }

  /**
   * Builder class for #MLTFeature
   *
   * @param <B> The final builder type, for covariance in property setters.
   * @param <C> The final feature type, for covariance in the build() method.
   */
  public abstract static class MLTFeatureBuilder<
          C extends MLTFeature, B extends MLTFeatureBuilder<C, B>>
      extends Feature.FeatureBuilder<C, B> {
    /**
     * Set properties from a Map of raw values, which will be adapted to Property objects.
     *
     * @param rawProperties A Map of String keys to Object values
     * @return The builder instance, for chaining
     */
    public B rawProperties(Map<String, Object> rawProperties) {
      return properties(
          rawProperties.entrySet().stream()
              .collect(Collectors.toMap(Map.Entry::getKey, MLTFeature::adapt)));
    }
  }
}
