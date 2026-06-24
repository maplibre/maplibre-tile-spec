import type IntWrapper from "../../decoding/intWrapper";
import { decodeVarintInt32 } from "../../decoding/integerDecodingUtils";
import type { Column, FeatureTableSchema, Field, TileSetMetadata } from "./tilesetMetadata";
import { columnTypeHasChildren, columnTypeHasName, decodeColumnType } from "./typeMap";

const textDecoder = new TextDecoder();

const SUPPORTED_COLUMN_TYPES = "0-3(ID), 4(GEOMETRY), 10-29(scalars), 30(STRUCT)";
const SUPPORTED_FIELD_TYPES = "10-29(scalars), 30(STRUCT)";

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
    const base = { name: column.name, nullable: column.nullable };
    return column.type === "scalarType"
        ? { ...base, type: "scalarField", scalarField: column.scalarType }
        : { ...base, type: "complexField", complexField: column.complexType };
}

/**
 * Decodes a Field used as part of complex types (STRUCT children).
 */
export function decodeField(src: Uint8Array, offset: IntWrapper): Field {
    const typeCode = decodeVarintInt32(src, offset, 1)[0] >>> 0;

    // Fields are only scalars (10-29) and STRUCT (30); ID/GEOMETRY codes (0-4) never appear as fields.
    // The lower bound rejects 0-9; decodeColumnType returns null for everything above 30.
    const base = typeCode >= 10 ? decodeColumnType(typeCode) : null;
    if (!base) {
        throw new Error(`Unsupported field type code ${typeCode}. Supported: ${SUPPORTED_FIELD_TYPES}`);
    }

    // Field type codes (10-30) always carry an explicit name.
    const column: Column = { ...base, name: decodeString(src, offset) };

    if (column.type === "complexType" && columnTypeHasChildren(typeCode)) {
        const complexCol = column.complexType;
        const childCount = decodeVarintInt32(src, offset, 1)[0] >>> 0;
        complexCol.children = new Array(childCount);
        for (let i = 0; i < childCount; i++) {
            complexCol.children[i] = decodeField(src, offset);
        }
    }

    return columnToField(column);
}

/**
 * The typeCode encodes the column type, nullable flag, and whether it has name/children.
 */
function decodeColumn(src: Uint8Array, offset: IntWrapper): Column {
    const typeCode = decodeVarintInt32(src, offset, 1)[0] >>> 0;
    const base = decodeColumnType(typeCode);

    if (!base) {
        throw new Error(`Unsupported column type code ${typeCode}. Supported: ${SUPPORTED_COLUMN_TYPES}`);
    }

    let name: string;
    if (columnTypeHasName(typeCode)) {
        name = decodeString(src, offset);
    } else if (typeCode <= 3) {
        // ID and GEOMETRY columns have implicit names
        name = "id";
    } else {
        name = "geometry";
    }

    const column: Column = { ...base, name };

    if (column.type === "complexType" && columnTypeHasChildren(typeCode)) {
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
    if (table.name.length === 0) {
        throw new Error("Missing layer name");
    }
    const extent = decodeVarintInt32(bytes, offset, 1)[0] >>> 0;

    const columnCount = decodeVarintInt32(bytes, offset, 1)[0] >>> 0;
    table.columns = new Array(columnCount);
    for (let j = 0; j < columnCount; j++) {
        table.columns[j] = decodeColumn(bytes, offset);
    }

    meta.featureTables.push(table);

    return [meta, extent];
}
