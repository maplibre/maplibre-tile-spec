package org.maplibre.mlt.converter.encodings;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.util.Set;
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.Test;
import org.maplibre.mlt.metadata.tileset.MltMetadata;

public class MltTypeMapTest {
  @Test
  public void roundTrips() {
    final var valid =
        Set.of(
            0, 1, 2, 3, 4, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
            28, 29, 30);

    for (var i = 0; i < 100; ++i) {
      final MltMetadata.FieldType[] types = new MltMetadata.FieldType[] {null};
      if (valid.contains(i)) {
        final var typeCode = i;
        Assertions.assertDoesNotThrow(
            () -> types[0] = MltTypeMap.Tag0x01.decodeColumnType(typeCode));
      } else {
        final var typeCode = i;
        Assertions.assertThrows(
            IllegalStateException.class,
            () -> {
              types[0] = MltTypeMap.Tag0x01.decodeColumnType(typeCode);
            });
        continue;
      }

      final var column = types[0];

      // STRUCT must have children before being re-encoded
      if (MltTypeMap.Tag0x01.columnTypeHasChildren(i)) {
        Assertions.assertNotNull(column.complexType);
        Assertions.assertNotNull(column.complexType.children);

        final var fieldType = MltMetadata.scalarFieldTypeBuilder(MltMetadata.ScalarType.STRING).isNullable(true).build();
        column.complexType.children.add(MltMetadata.Column.builder().type(fieldType).build());
      }

      final boolean complex = column.complexType != null;
      final boolean logical =
          (complex && column.complexType.logicalType != null)
              || (!complex && column.scalarType.logicalType != null);

      final var typeCode =
          MltTypeMap.Tag0x01.encodeColumnType(
                  (!complex && !logical) ? column.scalarType.physicalType : null,
                  (!complex && logical) ? column.scalarType.logicalType : null,
                  (complex && !logical) ? column.complexType.physicalType : null,
                  (complex && logical) ? column.complexType.logicalType : null,
                  column.isNullable,
                  complex && !column.complexType.children.isEmpty(),
                  !complex && column.scalarType.hasLongId)
              .or(Assertions::fail);
      assertEquals(typeCode.get(), i);
    }
  }
}
