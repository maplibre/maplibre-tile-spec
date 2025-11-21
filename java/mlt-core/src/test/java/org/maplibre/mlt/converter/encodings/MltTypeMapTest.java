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
      final MltMetadata.Column[] columns = new MltMetadata.Column[] {null};
      if (valid.contains(i)) {
        final var typeCode = i;
        Assertions.assertDoesNotThrow(
            () -> columns[0] = MltTypeMap.Tag0x01.decodeColumnType(typeCode));
      } else {
        final var typeCode = i;
        Assertions.assertThrows(
            IllegalStateException.class,
            () -> {
              columns[0] = MltTypeMap.Tag0x01.decodeColumnType(typeCode);
            });
        continue;
      }

      final var column = columns[0];

      // STRUCT must have children before being re-encoded
      if (MltTypeMap.Tag0x01.columnTypeHasChildren(i)) {
        column.complexType.children.add(
            new MltMetadata.Field(
                null, new MltMetadata.ScalarField(MltMetadata.ScalarType.STRING)));
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
