package org.maplibre.mlt.converter.encodings;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.util.List;
import java.util.Set;
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.Test;
import org.maplibre.mlt.metadata.tileset.MltMetadata;

/// Test encoding and decoding of column types
public class MltTypeMapTest {
  /// Test that all valid type codes can be decoded and re-encoded to the same value
  @Test
  public void roundTripsTag0x01() {
    final var valid =
        Set.of(
            0, 1, 2, 3, 4, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
            28, 29, 30);

    for (var i = 0; i < 2 * valid.size(); ++i) {
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

      var column = types[0];

      // STRUCT must have children before being re-encoded
      if (MltTypeMap.Tag0x01.columnTypeHasChildren(i)) {
        Assertions.assertNotNull(column.complexType());
        Assertions.assertNotNull(column.complexType().children());
        column =
            MltMetadata.structFieldType(
                List.of(
                    new MltMetadata.Field(
                        MltMetadata.scalarFieldType(MltMetadata.ScalarType.STRING, true))));
      }

      final boolean complex = column.complexType() != null;
      final boolean logical =
          (complex && column.complexType().logicalType() != null)
              || (!complex && column.scalarType().logicalType() != null);

      final var encodedTypeCode =
          MltTypeMap.Tag0x01.encodeColumnType(
              (!complex && !logical) ? column.scalarType().physicalType() : null,
              (!complex && logical) ? column.scalarType().logicalType() : null,
              (complex && !logical) ? column.complexType().physicalType() : null,
              (complex && logical) ? column.complexType().logicalType() : null,
              column.isNullable(),
              complex && !column.complexType().children().isEmpty(),
              !complex && column.scalarType().hasLongId());

      assertEquals(i, encodedTypeCode.orElseGet(Assertions::fail));
    }
  }

  @Test
  public void roundTripsTag0x02Map() {
    final var typeCodes = Set.of(31);
    for (var typeCode : typeCodes) {
      final var decoded = MltTypeMap.Tag0x02.decodeColumnType(typeCode);
      final var complex = decoded.complexType();
      Assertions.assertNotNull(complex);
      final var encoded =
          MltTypeMap.Tag0x02.encodeColumnType(
                  null,
                  null,
                  complex.physicalType(),
                  complex.logicalType(),
                  decoded.isNullable(),
                  !complex.children().isEmpty(),
                  false)
              .orElseGet(Assertions::fail);
      assertEquals(typeCode, encoded);
    }
  }
}
