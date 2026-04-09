package org.maplibre.mlt.metadata.tileset;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.List;
import org.junit.jupiter.api.Test;

class MltMetadataColumnBuilderTest {
  @Test
  void builderCreatesScalarColumn() {
    final var col =
            MltMetadata.Column.builder()
                    .type(MltMetadata.scalarFieldTypeBuilder(MltMetadata.ScalarType.STRING).isNullable(true).build())
            .name("name")
            .columnScope(MltMetadata.ColumnScope.FEATURE)
            .build()
            .toBuilder()
            .build();

    assertEquals("name", col.name);
    assertTrue(col.type.isNullable);
    assertEquals(MltMetadata.ColumnScope.FEATURE, col.columnScope);
    assertNotNull(col.type.scalarType);
    assertNull(col.type.complexType);
  }

  @Test
  void builderCreatesComplexColumn() {
    final var childType = MltMetadata.scalarFieldTypeBuilder(MltMetadata.ScalarType.INT_32).isNullable(false).build();
    final var child = MltMetadata.Column.builder().type(childType).build();
    final var columnType = MltMetadata.structFieldTypeBuilder(List.of(child)).build();
    final var col =
            MltMetadata.Column.builder().type(columnType)
            .name("geom")
            .columnScope(MltMetadata.ColumnScope.VERTEX)
            .build()
            .toBuilder()
            .build();

    assertEquals("geom", col.name);
    assertFalse(col.type.isNullable);
    assertEquals(MltMetadata.ColumnScope.VERTEX, col.columnScope);
    assertNotNull(col.type.complexType);
    assertNull(col.type.scalarType);
    assertEquals(MltMetadata.ComplexType.STRUCT, col.type.complexType.physicalType);
    assertEquals(1, col.type.complexType.children.size());
  }

  @Test
  void builderAcceptsNullChildren() {
    final List<MltMetadata.Field> children = null;
    final var colType = MltMetadata.complexFieldTypeBuilder(children).isNullable(true).build().toBuilder().build();
    final var col =
        MltMetadata.Column.builder().type(colType).build().toBuilder().build();

    assertNotNull(col.type.complexType);
    assertEquals(MltMetadata.ComplexType.STRUCT, col.type.complexType.physicalType);
    assertTrue(col.type.complexType.children.isEmpty());
  }

  @Test
  void builderDefaultsToFeatureScope() {
    final var col =
            MltMetadata.Column.builder()
                    .type(MltMetadata.scalarFieldTypeBuilder(MltMetadata.ScalarType.STRING).isNullable(true).build())
            .name("name")
            .build();
    assertEquals(MltMetadata.ColumnScope.FEATURE, col.columnScope);
  }

  @Test
  void builderRejectsInvalidScope() {
    final var builder =
            MltMetadata.Column.builder()
                    .type(MltMetadata.scalarFieldTypeBuilder(MltMetadata.ScalarType.STRING).isNullable(true).build())
            .name("invalid")
            .columnScope(MltMetadata.ColumnScope.UNRECOGNIZED);
    assertThrows(IllegalStateException.class, builder::build);
  }

  @Test
  void builderRejectsScalarAndComplexTypes() {
    final var builder = MltMetadata.scalarFieldTypeBuilder(MltMetadata.ScalarType.STRING)
            .complexType(new MltMetadata.ComplexField(MltMetadata.ComplexType.STRUCT))
            .isNullable(true);
    assertThrows(IllegalStateException.class, builder::build);
  }
}
