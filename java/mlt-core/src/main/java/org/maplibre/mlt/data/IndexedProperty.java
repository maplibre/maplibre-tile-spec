package org.maplibre.mlt.data;

import java.util.ArrayList;
import java.util.SequencedCollection;
import org.maplibre.mlt.metadata.tileset.MltMetadata;

public class IndexedProperty extends Property {
  private final SequencedCollection<Object> values;

  public IndexedProperty(
      MltMetadata.FieldType type, String name, SequencedCollection<Object> values) {
    super(type, name, null);
    this.values = values;
  }

  @Override
  public Object getValue(int featureIndex) {
    if (featureIndex < 0 || featureIndex >= values.size()) {
      throw new IndexOutOfBoundsException(
          "Index " + featureIndex + " is out of bounds for values of size " + values.size());
    }
    if (values instanceof ArrayList<?> arrayList) {
      return arrayList.get(featureIndex);
    }
    return values.stream().skip(featureIndex).findFirst().orElse(null);
  }
}
