package org.maplibre.mlt.metadata.tileset;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.List;
import org.junit.jupiter.api.Test;

class MltMetadataColumnTest {
  @Test
  void createScalarColumn() {
    final var col =
        new MltMetadata.Column(
            new MltMetadata.Field(
                MltMetadata.scalarFieldType(MltMetadata.ScalarType.STRING, true), "name"));
    assertEquals("name", col.getName());
    assertTrue(col.isNullable());
    assertEquals(MltMetadata.ColumnScope.FEATURE, col.columnScope());
    assertNotNull(col.field().type().scalarType());
    assertNull(col.field().type().complexType());
  }

  @Test
  void createComplexColumn() {
    final var child =
        new MltMetadata.Field(MltMetadata.scalarFieldType(MltMetadata.ScalarType.INT_32, false));
    final var col =
        new MltMetadata.Column(
            new MltMetadata.Field(MltMetadata.structFieldType(List.of(child)), "geom"),
            MltMetadata.ColumnScope.VERTEX);

    assertEquals("geom", col.getName());
    assertFalse(col.isNullable());
    assertEquals(MltMetadata.ColumnScope.VERTEX, col.columnScope());
    assertNotNull(col.field().type().complexType());
    assertNull(col.field().type().scalarType());
    assertEquals(MltMetadata.ComplexType.STRUCT, col.field().type().complexType().physicalType());
    assertEquals(1, col.field().type().complexType().children().size());
  }

  @Test
  void acceptsNullChildren() {
    final var col = new MltMetadata.Column(MltMetadata.structFieldType(null));
    assertNotNull(col.field().type().complexType());
    assertEquals(MltMetadata.ComplexType.STRUCT, col.field().type().complexType().physicalType());
    assertTrue(col.field().type().complexType().children().isEmpty());
  }

  @Test
  void defaultsToFeatureScope() {
    final var col =
        new MltMetadata.Column(MltMetadata.scalarFieldType(MltMetadata.ScalarType.STRING, true));
    assertEquals(MltMetadata.ColumnScope.FEATURE, col.columnScope());
  }

  @Test
  void rejectsInvalidScope() {
    assertThrows(
        IllegalStateException.class,
        () ->
            new MltMetadata.Column(
                new MltMetadata.Field(
                    MltMetadata.scalarFieldType(MltMetadata.ScalarType.STRING, true)),
                MltMetadata.ColumnScope.UNRECOGNIZED));
  }
}
