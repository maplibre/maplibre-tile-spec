package org.maplibre.mlt.data;

import org.maplibre.mlt.metadata.tileset.MltMetadata;

public class Property {
  private final boolean isNested;
  private final MltMetadata.ScalarType scalarType;
  private final String name;
  private final Object value;

  public Property(MltMetadata.ScalarType type, String name, Object value) {
    this.isNested = false;
    this.scalarType = type;
    this.name = name;
    this.value = value;
  }

  public Property(MltMetadata.FieldType type, String name, Object value) {
    this.isNested =
        (type.complexType != null && type.complexType.physicalType == MltMetadata.ComplexType.MAP);
    this.scalarType = MltMetadata.ScalarType.UNRECOGNIZED;
    this.name = name;
    this.value = value;
  }

  public boolean isNestedProperty() {
    return isNested;
  }

  public MltMetadata.ScalarType getType() {
    return scalarType;
  }

  public String getName() {
    return name;
  }

  public Object getValue() {
    return value;
  }
}
