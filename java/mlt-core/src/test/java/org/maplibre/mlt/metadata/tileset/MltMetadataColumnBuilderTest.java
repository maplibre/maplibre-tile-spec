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
        MltMetadata.columnBuilder()
            .name("name")
            .scalar(MltMetadata.ScalarType.STRING)
            .nullable(true)
            .scope(MltMetadata.ColumnScope.FEATURE)
            .build()
            .asColumnBuilder()
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
        MltMetadata.fieldBuilder().scalar(MltMetadata.ScalarType.INT_32).nullable(false).build();
    final var col =
        MltMetadata.columnBuilder()
            .name("geom")
            .struct(java.util.List.of(child))
            .nullable(false)
            .scope(MltMetadata.ColumnScope.VERTEX)
            .build()
            .asColumnBuilder()
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
    final var col = MltMetadata.columnBuilder().struct(null).build().asColumnBuilder().build();

    assertNotNull(col.complexType);
    assertEquals(MltMetadata.ComplexType.STRUCT, col.complexType.physicalType);
    assertTrue(col.complexType.children.isEmpty());
  }

  @Test
  void builderDefaultsToFeatureScope() {
    final var col =
        MltMetadata.columnBuilder()
            .name("name")
            .scalar(MltMetadata.ScalarType.STRING)
            .nullable(true)
            .build();
    assertEquals(MltMetadata.ColumnScope.FEATURE, col.columnScope);
  }

  @Test
  void builderRejectsInvalidScope() {
    final var builder =
        MltMetadata.columnBuilder()
            .name("invalid")
            .scalar(MltMetadata.ScalarType.STRING)
            .nullable(true)
            .scope(MltMetadata.ColumnScope.UNRECOGNIZED);
    assertThrows(IllegalStateException.class, builder::build);
  }

  @Test
  void builderRejectsNullScope() {
    final var builder =
        MltMetadata.columnBuilder()
            .name("invalid")
            .scalar(MltMetadata.ScalarType.STRING)
            .nullable(true)
            .scope(null);
    assertThrows(NullPointerException.class, builder::build);
  }

  @Test
  void builderRejectsScalarAndComplexTypes() {
    final var builder =
        MltMetadata.columnBuilder()
            .scalar(MltMetadata.ScalarType.INT_32)
            .struct(List.of())
            .scope(MltMetadata.ColumnScope.FEATURE);
    assertThrows(IllegalStateException.class, builder::build);
  }

  @Test
  void builderRejectsNoType() {
    final var builder =
        MltMetadata.columnBuilder()
            .name("invalid")
            .nullable(true)
            .scope(MltMetadata.ColumnScope.FEATURE);
    assertThrows(IllegalStateException.class, builder::build);
  }
}
