import type IntWrapper from "../../encodings/intWrapper";
import { decodeVarintInt32 } from "../../encodings/integerDecodingUtils";
import {
    type Column,
    type ComplexColumn,
    ComplexField,
    type ComplexType,
    FeatureTableSchema,
    Field,
    type LogicalComplexType,
    type LogicalScalarType,
    ScalarField,
    type ScalarType,
    TileSetMetadata,
} from "./tilesetMetadata.g";
import { TypeMap } from "./typeMap";

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

    const field = new Field();
    if ((fieldOptions & FieldOptions.nullable) !== 0) {
        field.nullable = true;
    }

    if (isComplex) {
        const complex = new ComplexField();
        complex.type = isLogical
            ? { case: "logicalType", value: typeValue as unknown as LogicalComplexType }
            : { case: "physicalType", value: typeValue as unknown as ComplexType };

        if ((fieldOptions & FieldOptions.hasChildren) !== 0) {
            const childCount = decodeVarintInt32(src, offset, 1)[0] >>> 0;
            complex.children = new Array(childCount);
            for (let i = 0; i < childCount; i++) {
                complex.children[i] = decodeField(src, offset);
            }
        }
        field.type = { case: "complexField", value: complex };
    } else {
        const scalar = new ScalarField();
        scalar.type = isLogical
            ? { case: "logicalType", value: typeValue as unknown as LogicalScalarType }
            : { case: "physicalType", value: typeValue as unknown as ScalarType };
        field.type = { case: "scalarField", value: scalar };
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
        const complexCol = column.type.value as ComplexColumn;
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
    const meta = new TileSetMetadata();
    meta.featureTables = [];

    const table = new FeatureTableSchema();
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
