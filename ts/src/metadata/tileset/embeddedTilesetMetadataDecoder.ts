
import {
    type Column,
    type ComplexField,
    type FeatureTableSchema,
    type Field,
    type ScalarField,
    type TileSetMetadata,
} from "./tilesetMetadata";
import { TypeMap } from "./typeMap";
import IntWrapper from "../../decoding/intWrapper";
import { decodeVarintInt32 } from "../../decoding/integerDecodingUtils";

const enum FieldOptions {
    nullable = 1 << 0,
    complexType = 1 << 1,
    logicalType = 1 << 2,
    hasChildren = 1 << 3,
}

const textDecoder = new TextDecoder();

/**
 * Decodes a length-prefixed UTF-8 string.
 * Layout: [len: varint32][bytes: len]
 */
function decodeString(src: Uint8Array, offset: IntWrapper): string {
    const length = decodeVarintInt32(src, offset, 1)[0];
    if (length === 0) {
        return "";
    }
    const start = offset.get();
    const end = start + length;
    const view = src.subarray(start, end);
    offset.add(length);
    return textDecoder.decode(view);
}

/**
 * Decodes a Field used as part of complex types (STRUCT children).
 * Unlike Column, Field still uses the fieldOptions bitfield for flexibility.
 */
function decodeField(src: Uint8Array, offset: IntWrapper): Field {
    const fieldOptions = decodeVarintInt32(src, offset, 1)[0] >>> 0;
    const isLogical = (fieldOptions & FieldOptions.logicalType) !== 0;
    const isComplex = (fieldOptions & FieldOptions.complexType) !== 0;

    const typeValue = decodeVarintInt32(src, offset, 1)[0] >>> 0;

    const field = {} as Field;
    if ((fieldOptions & FieldOptions.nullable) !== 0) {
        field.nullable = true;
    }

    if (isComplex) {
        const complex = {} as ComplexField;
        if (isLogical) {
            complex.type = "logicalType";
            complex.logicalType = typeValue;
        } else {
            complex.type = "physicalType";
            complex.physicalType = typeValue;
        }

        if ((fieldOptions & FieldOptions.hasChildren) !== 0) {
            const childCount = decodeVarintInt32(src, offset, 1)[0] >>> 0;
            complex.children = new Array(childCount);
            for (let i = 0; i < childCount; i++) {
                complex.children[i] = decodeField(src, offset);
            }
        }
        field.type = "complexField";
        field.complexField = complex;
    } else {
        const scalar = {} as ScalarField;
        if (isLogical) {
            scalar.type = "logicalType";
            scalar.logicalType = typeValue;
        } else {
            scalar.type = "physicalType";
            scalar.physicalType = typeValue;
        }
        field.type = "scalarField";
        field.scalarField = scalar;
    }

    return field;
}

/**
 * The typeCode encodes the column type, nullable flag, and whether it has name/children.
 */
function decodeColumn(src: Uint8Array, offset: IntWrapper): Column {
    const typeCode = decodeVarintInt32(src, offset, 1)[0] >>> 0;
    const column = TypeMap.decodeColumnType(typeCode);

    if (!column) {
        throw new Error(`Unsupported column type code: ${typeCode}`);
    }

    if (TypeMap.columnTypeHasName(typeCode)) {
        column.name = decodeString(src, offset);
    } else {
        // ID and GEOMETRY columns have implicit names
        if (typeCode >= 0 && typeCode <= 3) {
            column.name = "id";
        } else if (typeCode === 4) {
            column.name = "geometry";
        }
    }

    if (TypeMap.columnTypeHasChildren(typeCode)) {
        // Only STRUCT (typeCode 30) has children
        const childCount = decodeVarintInt32(src, offset, 1)[0] >>> 0;
        const complexCol = column.complexType;
        complexCol.children = new Array(childCount);
        for (let i = 0; i < childCount; i++) {
            complexCol.children[i] = decodeField(src, offset);
        }
    }

    return column;
}

/**
 * Top-level decoder for embedded tileset metadata.
 * Reads exactly ONE FeatureTableSchema from the stream.
 *
 * @param bytes The byte array containing the metadata
 * @param offset The current offset in the byte array (will be advanced)
 */
export function decodeEmbeddedTileSetMetadata(bytes: Uint8Array, offset: IntWrapper): [TileSetMetadata, number] {
    const meta = {} as TileSetMetadata;
    meta.featureTables = [];

    const table = {} as FeatureTableSchema;
    table.name = decodeString(bytes, offset);
    const extent = decodeVarintInt32(bytes, offset, 1)[0] >>> 0;

    const columnCount = decodeVarintInt32(bytes, offset, 1)[0] >>> 0;
    table.columns = new Array(columnCount);
    for (let j = 0; j < columnCount; j++) {
        table.columns[j] = decodeColumn(bytes, offset);
    }

    meta.featureTables.push(table);

    return [meta, extent];
}
