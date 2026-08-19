import {
    type Column,
    type ColumnWithoutName,
    ColumnScope,
    ComplexType,
    LogicalScalarType,
    ScalarType,
} from "./tilesetMetadata";

/**
 * The single varint32 that introduces every column in the tile metadata, identifying what kind of
 * column follows. Ids occupy a small range of flagged codes, geometry has one code of its own, and
 * scalar properties are laid out from `SCALAR_BASE` upwards, two codes per type.
 */
export const ColumnTypeCode = {
    /** Id columns occupy 0..3. */
    ID: 0,
    /** Set on an id column whose values can be null. */
    ID_NULLABLE: 1,
    /** Set on an id column holding 64-bit rather than 32-bit ids. */
    ID_LONG: 2,
    GEOMETRY: 4,
    /** Scalar properties are `SCALAR_BASE + scalarType * 2 + (nullable ? 1 : 0)`. */
    SCALAR_BASE: 10,
    STRUCT: 30,
    MAP: 31,
} as const;

/**
 * The type code is a single varint32 that encodes:
 * - Physical or logical type
 * - Nullable flag
 * - Whether the column has a name (typeCode >= ColumnTypeCode.SCALAR_BASE)
 * - Whether the column has children (typeCode == 30 for STRUCT)
 * - For ID types: whether it uses long (64-bit) IDs
 */

/**
 * Decodes a type code into a Column structure.
 *
 * ID type codes (0..3):
 * - Bit 0: nullable
 * - Bit 1: longID (0/1 -> uint32 IDs, 2/3 -> uint64 IDs)
 *
 * ID columns are kept as logical types so they remain distinguishable
 * from feature properties that may also be named "id".
 */
export function decodeColumnType(typeCode: number): ColumnWithoutName | null {
    switch (typeCode) {
        case ColumnTypeCode.ID:
        case ColumnTypeCode.ID | ColumnTypeCode.ID_NULLABLE:
        case ColumnTypeCode.ID | ColumnTypeCode.ID_LONG:
        case ColumnTypeCode.ID | ColumnTypeCode.ID_LONG | ColumnTypeCode.ID_NULLABLE:
            return {
                nullable: (typeCode & ColumnTypeCode.ID_NULLABLE) !== 0,
                columnScope: ColumnScope.FEATURE,
                type: "scalarType",
                scalarType: {
                    longID: (typeCode & ColumnTypeCode.ID_LONG) !== 0,
                    type: "logicalType",
                    logicalType: LogicalScalarType.ID,
                },
            };
        case ColumnTypeCode.GEOMETRY:
            // GEOMETRY (non-nullable, no children)
            return {
                nullable: false,
                columnScope: ColumnScope.FEATURE,
                type: "complexType",
                complexType: {
                    type: "physicalType",
                    physicalType: ComplexType.GEOMETRY,
                    children: [],
                },
            };
        case ColumnTypeCode.STRUCT:
            // STRUCT (non-nullable with children)
            return {
                nullable: false,
                columnScope: ColumnScope.FEATURE,
                type: "complexType",
                complexType: {
                    type: "physicalType",
                    physicalType: ComplexType.STRUCT,
                    children: [],
                },
            };
        case ColumnTypeCode.MAP:
            // MAP (nested properties, always nullable, may carry children when the
            // dictionaries are shared between sibling columns)
            return {
                nullable: true,
                columnScope: ColumnScope.FEATURE,
                type: "complexType",
                complexType: {
                    type: "physicalType",
                    physicalType: ComplexType.MAP,
                    children: [],
                },
            };
        default:
            return mapScalarType(typeCode);
    }
}

/**
 * Returns true if this type code requires a name to be stored.
 * ID (0-3) and GEOMETRY (4) columns have implicit names.
 * All other types (>= ColumnTypeCode.SCALAR_BASE) require explicit names.
 */
export function columnTypeHasName(typeCode: number): boolean {
    return typeCode >= ColumnTypeCode.SCALAR_BASE;
}

/**
 * Returns true if this type code has child fields.
 * STRUCT (typeCode 30) and MAP (typeCode 31) have children.
 */
export function columnTypeHasChildren(typeCode: number): boolean {
    return typeCode === ColumnTypeCode.STRUCT || typeCode === ColumnTypeCode.MAP;
}

/**
 * Determines if a stream count needs to be read for this column.
 * Mirrors the logic in cpp/include/mlt/metadata/type_map.hpp lines 85-122
 */
export function hasStreamCount(column: Column): boolean {
    if (column.type === "scalarType") {
        const scalarCol = column.scalarType;

        if (scalarCol.type === "physicalType") {
            const physicalType = scalarCol.physicalType;
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
        }
        if (scalarCol.type === "logicalType") {
            return false;
        }
    } else if (column.type === "complexType") {
        const complexCol = column.complexType;

        if (complexCol.type === "physicalType") {
            const physicalType = complexCol.physicalType;
            switch (physicalType) {
                case ComplexType.GEOMETRY:
                case ComplexType.STRUCT:
                case ComplexType.MAP:
                    return true;
                default:
                    return false;
            }
        }
    }

    console.warn("Unexpected column type in hasStreamCount", column);
    return false;
}

export function isLogicalIdColumn(column: Column): boolean {
    return (
        column.type === "scalarType" &&
        column.scalarType?.type === "logicalType" &&
        column.scalarType.logicalType === LogicalScalarType.ID
    );
}

export function isGeometryColumn(column: Column): boolean {
    return (
        column.type === "complexType" &&
        column.complexType?.type === "physicalType" &&
        column.complexType.physicalType === ComplexType.GEOMETRY
    );
}

/**
 * Maps a scalar type code to a Column with ScalarType.
 * Type codes 10-29 encode scalar types with nullable flag.
 * Even codes are non-nullable, odd codes are nullable.
 */
function mapScalarType(typeCode: number): ColumnWithoutName | null {
    let physicalType: number;

    switch (typeCode) {
        case 10:
        case 11:
            physicalType = ScalarType.BOOLEAN;
            break;
        case 12:
        case 13:
            physicalType = ScalarType.INT_8;
            break;
        case 14:
        case 15:
            physicalType = ScalarType.UINT_8;
            break;
        case 16:
        case 17:
            physicalType = ScalarType.INT_32;
            break;
        case 18:
        case 19:
            physicalType = ScalarType.UINT_32;
            break;
        case 20:
        case 21:
            physicalType = ScalarType.INT_64;
            break;
        case 22:
        case 23:
            physicalType = ScalarType.UINT_64;
            break;
        case 24:
        case 25:
            physicalType = ScalarType.FLOAT;
            break;
        case 26:
        case 27:
            physicalType = ScalarType.DOUBLE;
            break;
        case 28:
        case 29:
            physicalType = ScalarType.STRING;
            break;
        default:
            return null;
    }

    return {
        nullable: (typeCode & 1) !== 0,
        columnScope: ColumnScope.FEATURE,
        type: "scalarType",
        scalarType: {
            longID: false,
            type: "physicalType",
            physicalType,
        },
    };
}
