package org.maplibre.mlt.converter.encodings;

import jakarta.annotation.Nullable;
import java.util.Optional;
import org.maplibre.mlt.metadata.tileset.MltTilesetMetadata;

public class MltTypeMap {
  public static class Tag0x01 {
    /// Produces the unique type encoding for a `Column` or `Field`
    public static Optional<Integer> encodeColumnType(
        @Nullable MltTilesetMetadata.ScalarType physicalScalarType,
        @Nullable MltTilesetMetadata.LogicalScalarType logicalScalarType,
        @Nullable MltTilesetMetadata.ComplexType physicalComplexType,
        @Nullable MltTilesetMetadata.LogicalComplexType ignoredLogicalComplexType,
        boolean isNullable,
        boolean hasChildren,
        boolean hasLongIDs) {
      if (physicalScalarType != null) {
        if (!hasChildren) {
          return switch (physicalScalarType) {
            case BOOLEAN -> Optional.of(isNullable ? 11 : 10);
            case INT_8 -> Optional.of(isNullable ? 13 : 12);
            case UINT_8 -> Optional.of(isNullable ? 15 : 14);
            case INT_32 -> Optional.of(isNullable ? 17 : 16);
            case UINT_32 -> Optional.of(isNullable ? 19 : 18);
            case INT_64 -> Optional.of(isNullable ? 21 : 20);
            case UINT_64 -> Optional.of(isNullable ? 23 : 22);
            case FLOAT -> Optional.of(isNullable ? 25 : 24);
            case DOUBLE -> Optional.of(isNullable ? 27 : 26);
            case STRING -> Optional.of(isNullable ? 29 : 28);
            default -> Optional.empty();
          };
        }
      } else if (logicalScalarType != null) {
        if (logicalScalarType == MltTilesetMetadata.LogicalScalarType.ID) {
          return Optional.of(isNullable ? (hasLongIDs ? 3 : 1) : (hasLongIDs ? 2 : 0));
        }
      } else if (physicalComplexType != null) {
        if (physicalComplexType == MltTilesetMetadata.ComplexType.GEOMETRY) {
          if (!isNullable && !hasChildren) {
            return Optional.of(4);
          }
        } else if (physicalComplexType == MltTilesetMetadata.ComplexType.STRUCT) {
          if (!isNullable && hasChildren) {
            return Optional.of(30);
          }
        }
      }
      return Optional.empty();
    }

    /// Re-create a `Column` from the unique type code.
    /// The inverse of `getTag1TypeEncoding``
    public static MltTilesetMetadata.Column.Builder decodeColumnType(int typeCode) {
      final var builder = MltTilesetMetadata.Column.newBuilder();
      if (10 <= typeCode && typeCode <= 29) {
        return MltTilesetMetadata.Column.newBuilder()
            .setNullable((typeCode & 1) != 0)
            .setScalarType(
                MltTilesetMetadata.ScalarColumn.newBuilder()
                    .setPhysicalType(
                        switch (typeCode) {
                          case 10, 11 -> MltTilesetMetadata.ScalarType.BOOLEAN;
                          case 12, 13 -> MltTilesetMetadata.ScalarType.INT_8;
                          case 14, 15 -> MltTilesetMetadata.ScalarType.UINT_8;
                          case 16, 17 -> MltTilesetMetadata.ScalarType.INT_32;
                          case 18, 19 -> MltTilesetMetadata.ScalarType.UINT_32;
                          case 20, 21 -> MltTilesetMetadata.ScalarType.INT_64;
                          case 22, 23 -> MltTilesetMetadata.ScalarType.UINT_64;
                          case 24, 25 -> MltTilesetMetadata.ScalarType.FLOAT;
                          case 26, 27 -> MltTilesetMetadata.ScalarType.DOUBLE;
                          case 28, 29 -> MltTilesetMetadata.ScalarType.STRING;
                          default -> {
                            // Should be impossible due to the containing `if`
                            throw new IllegalStateException("Unsupported Type");
                          }
                        }));
      } else if (0 <= typeCode && typeCode <= 3) {
        return builder
            .setNullable((typeCode & 1) != 0)
            .setScalarType(
                MltTilesetMetadata.ScalarColumn.newBuilder()
                    .setLongID(typeCode > 1)
                    .setLogicalType(MltTilesetMetadata.LogicalScalarType.ID));
      } else if (4 == typeCode) {
        return builder
            .setNullable(false)
            .setComplexType(
                MltTilesetMetadata.ComplexColumn.newBuilder()
                    .setPhysicalType(MltTilesetMetadata.ComplexType.GEOMETRY));
      } else if (30 == typeCode) {
        return builder
            .setNullable(false)
            .setComplexType(
                MltTilesetMetadata.ComplexColumn.newBuilder()
                    .setPhysicalType(MltTilesetMetadata.ComplexType.STRUCT));
      } else {
        throw new IllegalStateException("Unsupported Type " + typeCode);
      }
    }

    public static boolean columnTypeHasName(int typeCode) {
      return (10 <= typeCode);
    }

    public static boolean columnTypeHasChildren(int typeCode) {
      return (typeCode == 30);
    }

    public static boolean isID(MltTilesetMetadata.Column column) {
      return column.hasScalarType()
          && column.getScalarType().hasLogicalType()
          && column.getScalarType().getLogicalType() == MltTilesetMetadata.LogicalScalarType.ID;
    }

    public static boolean isGeometry(MltTilesetMetadata.Column column) {
      return column.hasComplexType()
          && column.getComplexType().hasPhysicalType()
          && column.getComplexType().getPhysicalType() == MltTilesetMetadata.ComplexType.GEOMETRY;
    }

    static boolean isStruct(MltTilesetMetadata.Column column) {
      return column.hasComplexType()
          && column.getComplexType().hasPhysicalType()
          && column.getComplexType().getPhysicalType() == MltTilesetMetadata.ComplexType.STRUCT;
    }

    public static boolean hasStreamCount(MltTilesetMetadata.Column column) {
      if (column.hasScalarType()) {
        final var scalar = column.getScalarType();
        if (scalar.hasPhysicalType()) {
          switch (scalar.getPhysicalType()) {
            case BOOLEAN:
            case INT_8:
            case UINT_8:
            case INT_32:
            case UINT_32:
            case INT_64:
            case UINT_64:
            case FLOAT:
            case DOUBLE:
              return false;
            case STRING:
              return true;
            default:
          }
        } else if (scalar.hasLogicalType()) {
          if (scalar.getLogicalType() == MltTilesetMetadata.LogicalScalarType.ID) {
            return false;
          }
        }
      } else if (column.hasComplexType()) {
        final var complex = column.getComplexType();
        if (complex.hasPhysicalType()) {
          switch (complex.getPhysicalType()) {
            case GEOMETRY:
            case STRUCT:
              return true;
            default:
          }
        }
        // Complex/Logical not currently used
      }
      assert false;
      return false;
    }

    ///  Convert a decoded `Column` into `Field` for child types
    public static MltTilesetMetadata.Field toField(MltTilesetMetadata.Column column) {
      var field =
          MltTilesetMetadata.Field.newBuilder()
              .setNullable(column.getNullable())
              .setName(column.getName());
      if (column.hasScalarType()) {
        final var builder = MltTilesetMetadata.ScalarField.newBuilder();
        if (column.getScalarType().hasPhysicalType()) {
          field.setScalarField(builder.setPhysicalType(column.getScalarType().getPhysicalType()));
        } else if (column.getScalarType().hasLogicalType()) {
          field.setScalarField(builder.setLogicalType(column.getScalarType().getLogicalType()));
        } else {
          throw new RuntimeException("Unsupported Field Type");
        }
      } else if (column.hasComplexType()) {
        final var builder = MltTilesetMetadata.ComplexField.newBuilder();
        if (column.getComplexType().hasPhysicalType()) {
          field.setComplexField(builder.setPhysicalType(column.getComplexType().getPhysicalType()));
        } else if (column.getComplexType().hasLogicalType()) {
          field.setComplexField(
              builder.setLogicalType(column.getComplexTypeOrBuilder().getLogicalType()));
        } else {
          throw new RuntimeException("Unsupported Field Type");
        }
      } else {
        throw new RuntimeException("Unsupported Field Type");
      }
      return field.build();
    }
  }
}
