package org.maplibre.mlt.converter.encodings;

import java.util.Optional;
import javax.annotation.Nullable;
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
            case BOOLEAN -> Optional.of(isNullable ? 0 : 1);
            case INT_8 -> Optional.of(isNullable ? 2 : 3);
            case UINT_8 -> Optional.of(isNullable ? 4 : 5);
            case INT_32 -> Optional.of(isNullable ? 6 : 7);
            case UINT_32 -> Optional.of(isNullable ? 8 : 9);
            case INT_64 -> Optional.of(isNullable ? 10 : 11);
            case UINT_64 -> Optional.of(isNullable ? 12 : 13);
            case FLOAT -> Optional.of(isNullable ? 14 : 15);
            case DOUBLE -> Optional.of(isNullable ? 16 : 17);
            case STRING -> Optional.of(isNullable ? 18 : 19);
            default -> Optional.empty();
          };
        }
      } else if (logicalScalarType != null) {
        if (logicalScalarType == MltTilesetMetadata.LogicalScalarType.ID) {
          return Optional.of(isNullable ? (hasLongIDs ? 20 : 21) : (hasLongIDs ? 22 : 23));
        }
      } else if (physicalComplexType != null) {
        if (physicalComplexType == MltTilesetMetadata.ComplexType.GEOMETRY) {
          if (!isNullable && !hasChildren) {
            return Optional.of(24);
          }
        } else if (physicalComplexType == MltTilesetMetadata.ComplexType.STRUCT) {
          if (!isNullable && hasChildren) {
            return Optional.of(25);
          }
        }
      }
      return Optional.empty();
    }

    /// Re-create a `Column` from the unique type code.
    /// The inverse of `getTag1TypeEncoding``
    public static MltTilesetMetadata.Column.Builder decodeColumnType(int typeCode) {
      final var builder = MltTilesetMetadata.Column.newBuilder();
      if (0 <= typeCode && typeCode <= 19) {
        return MltTilesetMetadata.Column.newBuilder()
            .setNullable((typeCode & 1) == 0)
            .setScalarType(
                MltTilesetMetadata.ScalarColumn.newBuilder()
                    .setPhysicalType(
                        switch (typeCode) {
                          case 0, 1 -> MltTilesetMetadata.ScalarType.BOOLEAN;
                          case 2, 3 -> MltTilesetMetadata.ScalarType.INT_8;
                          case 4, 5 -> MltTilesetMetadata.ScalarType.UINT_8;
                          case 6, 7 -> MltTilesetMetadata.ScalarType.INT_32;
                          case 8, 9 -> MltTilesetMetadata.ScalarType.UINT_32;
                          case 10, 11 -> MltTilesetMetadata.ScalarType.INT_64;
                          case 12, 13 -> MltTilesetMetadata.ScalarType.UINT_64;
                          case 14, 15 -> MltTilesetMetadata.ScalarType.FLOAT;
                          case 16, 17 -> MltTilesetMetadata.ScalarType.DOUBLE;
                          case 18, 19 -> MltTilesetMetadata.ScalarType.STRING;
                          default -> {
                            // Should be impossible due to the containing `if`
                            throw new IllegalStateException("Unsupported Type");
                          }
                        }));
      } else if (20 <= typeCode && typeCode <= 23) {
        return builder
            .setNullable(typeCode < 22)
            .setName(PropertyEncoder.ID_COLUMN_NAME)
            .setScalarType(
                MltTilesetMetadata.ScalarColumn.newBuilder()
                    .setLongID((typeCode & 1) == 0)
                    .setLogicalType(MltTilesetMetadata.LogicalScalarType.ID));
      } else if (24 == typeCode) {
        return builder
            .setNullable(false)
            .setName(PropertyEncoder.GEOMETRY_COLUMN_NAME)
            .setComplexType(
                MltTilesetMetadata.ComplexColumn.newBuilder()
                    .setPhysicalType(MltTilesetMetadata.ComplexType.GEOMETRY));
      } else if (25 == typeCode) {
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
      return (typeCode < 20 || typeCode > 24);
    }

    public static boolean columnTypeHasChildren(int typeCode) {
      return (typeCode == 25);
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
