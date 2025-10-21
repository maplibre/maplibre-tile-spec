// based on ../spec/schema/mlt_tileset_metadata.proto
export const ColumnScope = {
    FEATURE: 0,
    VERTEX: 1,
} as const;

export const ScalarType = {
    BOOLEAN: 0,
    INT_8: 1,
    UINT_8: 2,
    INT_32: 3,
    UINT_32: 4,
    INT_64: 5,
    UINT_64: 6,
    FLOAT: 7,
    DOUBLE: 8,
    STRING: 9,
} as const;

export const ComplexType = {
    GEOMETRY: 0,
    STRUCT: 1,
} as const;

export const LogicalScalarType = {
    ID: 0,
} as const;

export const LogicalComplexType = {
    BINARY: 0,
    RANGE_MAP: 1,
} as const;

export interface TileSetMetadata {
    version?: number | null;
    featureTables: FeatureTableSchema[];
    name?: string | null;
    description?: string | null;
    attribution?: string | null;
    minZoom?: number | null;
    maxZoom?: number | null;
    bounds: number[];
    center: number[];
}

export interface FeatureTableSchema {
    name?: string | null;
    columns: Column[];
}

export interface Column {
    name?: string | null;
    nullable?: boolean | null;
    columnScope?: number | null;
    scalarType?: ScalarColumn | null;
    complexType?: ComplexColumn | null;
    type?: "scalarType" | "complexType";
}

export interface ScalarColumn {
    longID?: boolean | null;
    physicalType?: number | null;
    logicalType?: number | null;
    type?: "physicalType" | "logicalType";
}

export interface ComplexColumn {
    physicalType?: number | null;
    logicalType?: number | null;
    children: Field[];
    type?: "physicalType" | "logicalType";
}

export interface Field {
    name?: string | null;
    nullable?: boolean | null;
    scalarField?: ScalarField | null;
    complexField?: ComplexField | null;
    type?: "scalarField" | "complexField";
}

export interface ScalarField {
    physicalType?: number | null;
    logicalType?: number | null;
    type?: "physicalType" | "logicalType";
}

export interface ComplexField {
    physicalType?: number | null;
    logicalType?: number | null;
    children: Field[];
    type?: "physicalType" | "logicalType";
}
