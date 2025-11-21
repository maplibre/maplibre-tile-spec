package org.maplibre.mlt.converter.encodings;

import jakarta.annotation.Nullable;
import java.util.Optional;
import org.maplibre.mlt.metadata.tileset.MltMetadata;

public class MltTypeMap {
  public static class Tag0x01 {
    /// Produces the unique type encoding for a `Column` or `Field`
    public static Optional<Integer> encodeColumnType(
        @Nullable MltMetadata.ScalarType physicalScalarType,
        @Nullable MltMetadata.LogicalScalarType logicalScalarType,
        @Nullable MltMetadata.ComplexType physicalComplexType,
        @Nullable MltMetadata.LogicalComplexType ignoredLogicalComplexType,
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
        if (logicalScalarType == MltMetadata.LogicalScalarType.ID) {
          return Optional.of(isNullable ? (hasLongIDs ? 3 : 1) : (hasLongIDs ? 2 : 0));
        }
      } else if (physicalComplexType != null) {
        if (physicalComplexType == MltMetadata.ComplexType.GEOMETRY) {
          if (!isNullable && !hasChildren) {
            return Optional.of(4);
          }
        } else if (physicalComplexType == MltMetadata.ComplexType.STRUCT) {
          if (!isNullable && hasChildren) {
            return Optional.of(30);
          }
        }
      }
      return Optional.empty();
    }

    /// Re-create a `Column` from the unique type code.
    /// The inverse of `getTag1TypeEncoding``
    public static MltMetadata.Column decodeColumnType(int typeCode) {
      if (10 <= typeCode && typeCode <= 29) {
        final var column =
            new MltMetadata.Column(null, new MltMetadata.ScalarField(getScalarType(typeCode)));
        column.isNullable = (typeCode & 1) != 0;
        return column;
      } else if (0 <= typeCode && typeCode <= 3) {
        final var column =
            new MltMetadata.Column(
                null, new MltMetadata.ScalarField(MltMetadata.LogicalScalarType.ID));
        column.isNullable = (typeCode & 1) != 0;
        column.scalarType.hasLongId = (typeCode > 1);
        return column;
      } else if (4 == typeCode) {
        final var column =
            new MltMetadata.Column(
                null, new MltMetadata.ComplexField(MltMetadata.ComplexType.GEOMETRY));
        column.isNullable = false;
        return column;
      } else if (30 == typeCode) {
        final var column =
            new MltMetadata.Column(
                null, new MltMetadata.ComplexField(MltMetadata.ComplexType.STRUCT));
        column.isNullable = false;
        return column;
      } else {
        throw new IllegalStateException("Unsupported Type " + typeCode);
      }
    }

    private static MltMetadata.ScalarType getScalarType(int typeCode) {
      return switch (typeCode) {
        case 10, 11 -> MltMetadata.ScalarType.BOOLEAN;
        case 12, 13 -> MltMetadata.ScalarType.INT_8;
        case 14, 15 -> MltMetadata.ScalarType.UINT_8;
        case 16, 17 -> MltMetadata.ScalarType.INT_32;
        case 18, 19 -> MltMetadata.ScalarType.UINT_32;
        case 20, 21 -> MltMetadata.ScalarType.INT_64;
        case 22, 23 -> MltMetadata.ScalarType.UINT_64;
        case 24, 25 -> MltMetadata.ScalarType.FLOAT;
        case 26, 27 -> MltMetadata.ScalarType.DOUBLE;
        case 28, 29 -> MltMetadata.ScalarType.STRING;
        default -> throw new IllegalStateException("Unsupported Type");
      };
    }

    public static boolean columnTypeHasName(int typeCode) {
      return (10 <= typeCode);
    }

    public static boolean columnTypeHasChildren(int typeCode) {
      return (typeCode == 30);
    }

    public static boolean isID(MltMetadata.Column column) {
      return column.scalarType != null
          && column.scalarType.logicalType == MltMetadata.LogicalScalarType.ID;
    }

    public static boolean isGeometry(MltMetadata.Column column) {
      return column.complexType != null
          && column.complexType.physicalType == MltMetadata.ComplexType.GEOMETRY;
    }

    static boolean isStruct(MltMetadata.Column column) {
      return column.complexType != null
          && column.complexType.physicalType == MltMetadata.ComplexType.STRUCT;
    }

    public static boolean hasStreamCount(MltMetadata.Column column) {
      if (column.scalarType != null) {
        final var scalar = column.scalarType;
        if (scalar.physicalType != null) {
          switch (scalar.physicalType) {
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
        } else if (scalar.logicalType != null) {
          if (scalar.logicalType == MltMetadata.LogicalScalarType.ID) {
            return false;
          }
        }
      } else if (column.complexType != null) {
        final var complex = column.complexType;
        if (complex.physicalType != null) {
          switch (complex.physicalType) {
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
  }
}
