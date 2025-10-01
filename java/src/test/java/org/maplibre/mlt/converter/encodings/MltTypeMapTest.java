package org.maplibre.mlt.converter.encodings;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.util.Set;
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.Test;
import org.maplibre.mlt.metadata.tileset.MltTilesetMetadata;

public class MltTypeMapTest {
  @Test
  public void roundTrips() {
    final var valid =
        Set.of(
            0, 1, 2, 3, 4, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
            28, 29, 30);

    for (var i = 0; i < 100; ++i) {
      final MltTilesetMetadata.Column.Builder[] columns =
          new MltTilesetMetadata.Column.Builder[] {null};
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
        column.setComplexType(
            MltTilesetMetadata.ComplexColumn.newBuilder(column.getComplexType())
                .addChildren(MltTilesetMetadata.Field.newBuilder()));
      }

      final boolean complex = column.hasComplexType();
      final boolean logical =
          (complex && column.getComplexType().hasLogicalType())
              || (!complex && column.getScalarType().hasLogicalType());

      final var typeCode =
          MltTypeMap.Tag0x01.encodeColumnType(
                  (!complex && !logical) ? column.getScalarType().getPhysicalType() : null,
                  (!complex && logical) ? column.getScalarType().getLogicalType() : null,
                  (complex && !logical) ? column.getComplexType().getPhysicalType() : null,
                  (complex && logical) ? column.getComplexType().getLogicalType() : null,
                  column.getNullable(),
                  complex && !column.getComplexType().getChildrenList().isEmpty(),
                  !complex && column.getScalarType().getLongID())
              .or(Assertions::fail);
      assertEquals(typeCode.get(), i);
    }
  }
}
