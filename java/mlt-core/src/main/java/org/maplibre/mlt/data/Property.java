package org.maplibre.mlt.data;

import java.util.Objects;
import lombok.Getter;
import org.jetbrains.annotations.NotNull;
import org.maplibre.mlt.metadata.tileset.MltMetadata;

/** Represents a property of a feature */
public class Property {
  @Getter @NotNull private final MltMetadata.FieldType type;
  @Getter private final String name;
  private final Object value;

  /**
   * Create directly from a complex type
   *
   * @param type The type of the property
   * @param name The name of the property
   * @param value The value of the property, which should be compatible with the type. No validation
   *     is performed.
   */
  public Property(@NotNull MltMetadata.FieldType type, String name, Object value) {
    this.type = Objects.requireNonNull(type);
    this.name = name;
    this.value = value;

    if (!isNested(type) && value != null) {
      if (type.scalarType() == null
          || type.scalarType().physicalType().ordinal() < MltMetadata.ScalarType.BOOLEAN.ordinal()
          || type.scalarType().physicalType().ordinal() > MltMetadata.ScalarType.STRING.ordinal()) {
        throw new IllegalArgumentException(
            "FieldType must have either a valid scalarType or a complexType of MAP");
      }
    }
  }

  /**
   * @param featureIndex The index of the feature for which to get the value
   * @return The value of the property.
   */
  public Object getValue(int featureIndex) {
    return value;
  }

  private static boolean isNested(MltMetadata.FieldType type) {
    return (type.complexType() != null
        && type.complexType().physicalType() == MltMetadata.ComplexType.MAP);
  }

  /**
   * @return True if the property is nested
   */
  public boolean isNested() {
    return isNested(type);
  }
}
