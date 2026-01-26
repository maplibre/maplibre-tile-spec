import type IntWrapper from "../../decoding/intWrapper";
import { decodeVarintInt32 } from "../../decoding/integerDecodingUtils";
import { type Column, type FeatureTableSchema, type Field, type TileSetMetadata } from "./tilesetMetadata";
import { columnTypeHasChildren, columnTypeHasName, decodeColumnType } from "./typeMap";

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
 * Converts a Column to a Field.
 * Used when decoding Field metadata which has the same format as Column.
 */
function columnToField(column: Column): Field {
    return {
        name: column.name,
        nullable: column.nullable,
        scalarField: column.scalarType,
        complexField: column.complexType,
        type: column.type === "scalarType" ? "scalarField" : "complexField",
    };
}

/**
 * Decodes a Field used as part of complex types (STRUCT children).
 */
export function decodeField(src: Uint8Array, offset: IntWrapper): Field {
    const typeCode = decodeVarintInt32(src, offset, 1)[0] >>> 0;
    const column = decodeColumnType(typeCode);

    if (!column) {
        throw new Error(`Unsupported field type code: ${typeCode}`);
    }

    if (columnTypeHasName(typeCode)) {
        column.name = decodeString(src, offset);
    }

    if (columnTypeHasChildren(typeCode)) {
        const childCount = decodeVarintInt32(src, offset, 1)[0] >>> 0;
        column.complexType.children = new Array(childCount);
        for (let i = 0; i < childCount; i++) {
            column.complexType.children[i] = decodeField(src, offset);
        }
    }

    return columnToField(column);
}

/**
 * The typeCode encodes the column type, nullable flag, and whether it has name/children.
 */
function decodeColumn(src: Uint8Array, offset: IntWrapper): Column {
    const typeCode = decodeVarintInt32(src, offset, 1)[0] >>> 0;
    const column = decodeColumnType(typeCode);

    if (!column) {
        throw new Error(`Unsupported column type code: ${typeCode}`);
    }

    if (columnTypeHasName(typeCode)) {
        column.name = decodeString(src, offset);
    } else {
        // ID and GEOMETRY columns have implicit names
        if (typeCode >= 0 && typeCode <= 3) {
            column.name = "id";
        } else if (typeCode === 4) {
            column.name = "geometry";
        }
    }

    if (columnTypeHasChildren(typeCode)) {
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
