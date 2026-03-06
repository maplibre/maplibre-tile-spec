export enum ColumnDataType {
    STRING = 0,
    FLOAT = 1,
    DOUBLE = 2,
    INT_64 = 3,
    UINT_64 = 4,
    BOOLEAN = 5,
    GEOMETRY = 6,
    GEOMETRY_M = 7,
    GEOMETRY_Z = 8,
    GEOMETRY_ZM = 9,
}

export enum ColumnEncoding {
    /*
     * String -> no dictionary coding
     * Geometry -> standard unsorted encoding
     * */
    PLAIN = 0,
    VARINT = 1,
    DELTA_VARINT = 2,
    RLE = 3,
    BOOLEAN_RLE = 4,
    BYTE_RLE = 5,
    DICTIONARY = 6,
    LOCALIZED_DICTIONARY = 7,
    ORDERED_GEOMETRY_ENCODING = 8,
    INDEXED_COORDINATE_ENCODING = 9,
}

export interface ColumnMetadata {
    columnName: string;
    columnType: ColumnDataType;
    columnEncoding: ColumnEncoding;
    streams: Map<string, StreamMetadata>;
}

export interface StreamMetadata {
    numValues: number;
    byteLength: number;
}

export interface LayerMetadata {
    name: string;
    numColumns: number;
    numFeatures: number;
    columnMetadata: ColumnMetadata[];
}
