package org.maplibre.mlt.data;

import lombok.Getter;
import org.maplibre.mlt.metadata.tileset.MltMetadata;

public class Property {
  @Getter private final MltMetadata.FieldType type;
  @Getter private final String name;
  private final Object value;

  public Property(MltMetadata.ScalarType type, String name, Object value) {
    this(MltMetadata.scalarFieldBuilder(type).build(), name, value);
  }

  public Property(MltMetadata.FieldType type, String name, Object value) {
    this.type = type;
    this.name = name;
    this.value = value;

    if (!isNested() && value != null) {
      if (type.scalarType == null
          || type.scalarType.physicalType.ordinal() < MltMetadata.ScalarType.BOOLEAN.ordinal()
          || type.scalarType.physicalType.ordinal() > MltMetadata.ScalarType.STRING.ordinal()) {
        throw new IllegalArgumentException(
            "FieldType must have either a valid scalarType or a complexType of MAP");
      }
    }
  }

  public Object getValue(int featureIndex) {
    return value;
  }

  public boolean isNested() {
    return (type.complexType != null
        && type.complexType.physicalType == MltMetadata.ComplexType.MAP);
  }
}
