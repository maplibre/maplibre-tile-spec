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
        MltMetadata.scalarColumnBuilder(MltMetadata.ScalarType.STRING)
            .name("name")
            .isNullable(true)
            .columnScope(MltMetadata.ColumnScope.FEATURE)
            .build()
            .toBuilder()
            .build();

    assertEquals("name", col.name);
    assertTrue(col.isNullable);
    assertEquals(MltMetadata.ColumnScope.FEATURE, col.columnScope);
    assertNotNull(col.scalarType);
    assertNull(col.complexType);
  }

  @Test
  void builderCreatesComplexColumn() {
    final var child =
        MltMetadata.scalarFieldBuilder(MltMetadata.ScalarType.INT_32).isNullable(false).build();
    final var col =
        MltMetadata.structColumnBuilder(List.of(child))
            .name("geom")
            .isNullable(false)
            .columnScope(MltMetadata.ColumnScope.VERTEX)
            .build()
            .toBuilder()
            .build();

    assertEquals("geom", col.name);
    assertFalse(col.isNullable);
    assertEquals(MltMetadata.ColumnScope.VERTEX, col.columnScope);
    assertNotNull(col.complexType);
    assertNull(col.scalarType);
    assertEquals(MltMetadata.ComplexType.STRUCT, col.complexType.physicalType);
    assertEquals(1, col.complexType.children.size());
  }

  @Test
  void builderAcceptsNullChildren() {
    final var col =
        MltMetadata.structColumnBuilder(null).isNullable(true).build().toBuilder().build();

    assertNotNull(col.complexType);
    assertEquals(MltMetadata.ComplexType.STRUCT, col.complexType.physicalType);
    assertTrue(col.complexType.children.isEmpty());
  }

  @Test
  void builderDefaultsToFeatureScope() {
    final var col =
        MltMetadata.scalarColumnBuilder(MltMetadata.ScalarType.STRING)
            .name("name")
            .isNullable(true)
            .build();
    assertEquals(MltMetadata.ColumnScope.FEATURE, col.columnScope);
  }

  @Test
  void builderRejectsInvalidScope() {
    final var builder =
        MltMetadata.scalarColumnBuilder(MltMetadata.ScalarType.STRING)
            .name("invalid")
            .isNullable(true)
            .columnScope(MltMetadata.ColumnScope.UNRECOGNIZED);
    assertThrows(IllegalStateException.class, builder::build);
  }

  @Test
  void builderRejectsScalarAndComplexTypes() {
    final var builder =
        MltMetadata.scalarColumnBuilder(MltMetadata.ScalarType.STRING)
            .complexType(new MltMetadata.ComplexField(MltMetadata.ComplexType.STRUCT))
            .columnScope(MltMetadata.ColumnScope.FEATURE);
    assertThrows(IllegalStateException.class, builder::build);
  }
}
