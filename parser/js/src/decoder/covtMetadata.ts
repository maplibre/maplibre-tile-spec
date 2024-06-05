export enum ColumnDataType {
    STRING,
    FLOAT,
    DOUBLE,
    INT_64,
    UINT_64,
    BOOLEAN,
    GEOMETRY,
    GEOMETRY_M,
    GEOMETRY_Z,
    GEOMETRY_ZM,
}

export enum ColumnEncoding {
    /*
     * String -> no dictionary coding
     * Geometry -> standard unsorted encoding
     * */
    PLAIN,
    VARINT,
    DELTA_VARINT,
    RLE,
    BOOLEAN_RLE,
    BYTE_RLE,
    DICTIONARY,
    LOCALIZED_DICTIONARY,
    ORDERED_GEOMETRY_ENCODING,
    INDEXED_COORDINATE_ENCODING,
}

export interface ColumnMetadata {
    name: string;
    type: ColumnDataType;
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
