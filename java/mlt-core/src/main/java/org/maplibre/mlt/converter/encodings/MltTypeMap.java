package org.maplibre.mlt.converter.encodings;

import jakarta.annotation.Nullable;
import java.util.List;
import java.util.Optional;
import org.maplibre.mlt.metadata.tileset.MltMetadata;

public class MltTypeMap {
  public static class Tag0x01 {
    /// Produces the unique type encoding for a `Column` or `Field`
    /// @param physicalScalarType The physical scalar type, if applicable
    /// @param logicalScalarType The logical scalar type, if applicable
    /// @param physicalComplexType The physical complex type, if applicable
    /// @param ignoredLogicalComplexType not currently used
    /// @param isNullable Whether the column is nullable
    /// @param hasChildren Whether the column has children (i.e. is a struct)
    /// @param hasLongIDs Whether the column has long IDs (i.e. 64-bit vs 32-bit)
    /// @return A unique integer encoding of the column type, if supported by the encoding scheme
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
    /// @param typeCode the unique type code encoding the column's type
    /// @return The equivalent type descriptor
    public static MltMetadata.FieldType decodeColumnType(int typeCode) {
      if (10 <= typeCode && typeCode <= 29) {
        final var isNullable = (typeCode & 1) != 0;
        return MltMetadata.scalarFieldType(getScalarType(typeCode), isNullable);
      } else if (0 <= typeCode && typeCode <= 3) {
        final var isNullable = (typeCode & 1) != 0;
        final var hasLongId = (typeCode > 1);
        return MltMetadata.idFieldType(hasLongId, isNullable);
      } else if (4 == typeCode) {
        return MltMetadata.geometryFieldType();
      } else if (30 == typeCode) {
        return MltMetadata.structFieldType((List<MltMetadata.Field>) null);
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
      return column.is(MltMetadata.LogicalScalarType.ID);
    }

    public static boolean isGeometry(MltMetadata.Column column) {
      return column.is(MltMetadata.ComplexType.GEOMETRY);
    }

    static boolean isStruct(MltMetadata.Column column) {
      return column.is(MltMetadata.ComplexType.STRUCT);
    }

    public static boolean hasStreamCount(MltMetadata.Column column) {
      if (column.isScalar()) {
        final var scalar = column.field().type().scalarType();
        if (scalar.physicalType() != null) {
          switch (scalar.physicalType()) {
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
        } else if (scalar.logicalType() != null) {
          if (scalar.logicalType() == MltMetadata.LogicalScalarType.ID) {
            return false;
          }
        }
      } else if (column.isComplex()) {
        final var complex = column.field().type().complexType();
        if (complex.physicalType() != null) {
          switch (complex.physicalType()) {
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
