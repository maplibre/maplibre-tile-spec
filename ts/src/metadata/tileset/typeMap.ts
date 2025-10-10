import {
    Column,
    ColumnScope,
    ComplexColumn,
    ComplexType,
    LogicalScalarType,
    ScalarColumn,
    ScalarType,
} from "./tilesetMetadata";

/**
 * The type code is a single varint32 that encodes:
 * - Physical or logical type
 * - Nullable flag
 * - Whether the column has a name (typeCode >= 10)
 * - Whether the column has children (typeCode == 30 for STRUCT)
 * - For ID types: whether it uses long (64-bit) IDs
 */
export class TypeMap {
    /**
     * Decodes a type code into a Column structure.
     * ID columns (0-3) are represented as physical UINT_32 or UINT_64 types in TypeScript
     */
    static decodeColumnType(typeCode: number): Column | null {
        switch (typeCode) {
            case 0:
            case 1:
            case 2:
            case 3: {
                // ID columns: 0=uint32, 1=uint64, 2=nullable uint32, 3=nullable uint64
                const column = new Column();
                column.nullable = typeCode > 1;
                column.columnScope = ColumnScope.FEATURE;
                const scalarCol = new ScalarColumn();
                // Map to physical type since TS schema doesn't have LogicalScalarType.ID
                const physicalType = (typeCode & 1) !== 0 ? ScalarType.UINT_64 : ScalarType.UINT_32;
                scalarCol.type = { case: "physicalType", value: physicalType };
                column.type = { case: "scalarType", value: scalarCol };
                return column;
            }
            case 4: {
                // GEOMETRY (non-nullable, no children)
                const column = new Column();
                column.nullable = false;
                column.columnScope = ColumnScope.FEATURE;
                const complexCol = new ComplexColumn();
                complexCol.type = { case: "physicalType", value: ComplexType.GEOMETRY };
                column.type = { case: "complexType", value: complexCol };
                return column;
            }
            case 30: {
                // STRUCT (non-nullable with children)
                const column = new Column();
                column.nullable = false;
                column.columnScope = ColumnScope.FEATURE;
                const complexCol = new ComplexColumn();
                complexCol.type = { case: "physicalType", value: ComplexType.STRUCT };
                column.type = { case: "complexType", value: complexCol };
                return column;
            }
            default:
                return this.mapScalarType(typeCode);
        }
    }

    /**
     * Returns true if this type code requires a name to be stored.
     * ID (0-3) and GEOMETRY (4) columns have implicit names.
     * All other types (>= 10) require explicit names.
     */
    static columnTypeHasName(typeCode: number): boolean {
        return typeCode >= 10;
    }

    /**
     * Returns true if this type code has child fields.
     * Only STRUCT (typeCode 30) has children.
     */
    static columnTypeHasChildren(typeCode: number): boolean {
        return typeCode === 30;
    }

    /**
     * Determines if a stream count needs to be read for this column.
     * Mirrors the logic in cpp/include/mlt/metadata/type_map.hpp lines 81-118
     */
    static hasStreamCount(column: Column): boolean {
        // ID columns don't have stream count (identified by name)
        if (column.name === "id") {
            return false;
        }

        if (column.type.case === "scalarType") {
            const scalarCol = column.type.value as ScalarColumn;

            if (scalarCol.type.case === "physicalType") {
                const physicalType = scalarCol.type.value as ScalarType;
                switch (physicalType) {
                    case ScalarType.BOOLEAN:
                    case ScalarType.INT_8:
                    case ScalarType.UINT_8:
                    case ScalarType.INT_32:
                    case ScalarType.UINT_32:
                    case ScalarType.INT_64:
                    case ScalarType.UINT_64:
                    case ScalarType.FLOAT:
                    case ScalarType.DOUBLE:
                        return false;
                    case ScalarType.STRING:
                        return true;
                    default:
                        return false;
                }
            } else if (scalarCol.type.case === "logicalType") {
                return false;
            }
        } else if (column.type.case === "complexType") {
            const complexCol = column.type.value as ComplexColumn;

            if (complexCol.type.case === "physicalType") {
                const physicalType = complexCol.type.value as ComplexType;
                switch (physicalType) {
                    case ComplexType.GEOMETRY:
                    case ComplexType.STRUCT:
                        return true;
                    default:
                        return false;
                }
            }
        }

        console.warn("Unexpected column type in hasStreamCount", column);
        return false;
    }

    /**
     * Maps a scalar type code to a Column with ScalarType.
     * Type codes 10-29 encode scalar types with nullable flag.
     * Even codes are non-nullable, odd codes are nullable.
     */
    private static mapScalarType(typeCode: number): Column | null {
        let scalarType: ScalarType | null = null;

        switch (typeCode) {
            case 10:
            case 11:
                scalarType = ScalarType.BOOLEAN;
                break;
            case 12:
            case 13:
                scalarType = ScalarType.INT_8;
                break;
            case 14:
            case 15:
                scalarType = ScalarType.UINT_8;
                break;
            case 16:
            case 17:
                scalarType = ScalarType.INT_32;
                break;
            case 18:
            case 19:
                scalarType = ScalarType.UINT_32;
                break;
            case 20:
            case 21:
                scalarType = ScalarType.INT_64;
                break;
            case 22:
            case 23:
                scalarType = ScalarType.UINT_64;
                break;
            case 24:
            case 25:
                scalarType = ScalarType.FLOAT;
                break;
            case 26:
            case 27:
                scalarType = ScalarType.DOUBLE;
                break;
            case 28:
            case 29:
                scalarType = ScalarType.STRING;
                break;
            default:
                return null;
        }

        const column = new Column();
        column.nullable = (typeCode & 1) !== 0;
        column.columnScope = ColumnScope.FEATURE;
        const scalarCol = new ScalarColumn();
        scalarCol.type = { case: "physicalType", value: scalarType };
        column.type = { case: "scalarType", value: scalarCol };
        return column;
    }
}