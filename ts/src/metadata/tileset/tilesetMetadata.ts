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
    version?: number;
    featureTables: FeatureTableSchema[];
    name?: string;
    description?: string;
    attribution?: string;
    minZoom?: number;
    maxZoom?: number;
    bounds: number[];
    center: number[];
}

export interface FeatureTableSchema {
    name: string;
    columns: Column[];
}

export interface Column {
    name: string;
    nullable: boolean;
    columnScope: number;
    scalarType?: ScalarColumn;
    complexType?: ComplexColumn;
    type?: "scalarType" | "complexType";
}

export interface ScalarColumn {
    longID: boolean;
    physicalType?: number;
    logicalType?: number;
    type?: "physicalType" | "logicalType";
}

export interface ComplexColumn {
    physicalType?: number;
    logicalType?: number;
    children: Field[];
    type?: "physicalType" | "logicalType";
}

export interface Field {
    name?: string;
    nullable?: boolean;
    scalarField?: ScalarField;
    complexField?: ComplexField;
    type?: "scalarField" | "complexField";
}

export interface ScalarField {
    physicalType?: number;
    logicalType?: number;
    type?: "physicalType" | "logicalType";
}

export interface ComplexField {
    physicalType?: number;
    logicalType?: number;
    children: Field[];
    type?: "physicalType" | "logicalType";
}
