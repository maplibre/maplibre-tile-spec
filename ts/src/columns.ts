import { decodeRawTileLayers } from "./decoding/v2/layer";
import { isBitSet } from "./decoding/v2/presence";

export enum MltGeometryType {
    Point = 0,
    LineString = 1,
    Polygon = 2,
    MultiPoint = 3,
    MultiLineString = 4,
    MultiPolygon = 5,
    Mesh = 6,
}

export type HeightMode = "absolute" | "relativeToGround";
export type MltHeightMode = HeightMode;
export type MltGeometryLayout = "outlines" | "tessPolygons" | "tessPolygonsWithOutlines" | "meshes";

export interface MltGeometryColumns {
    readonly types: Uint8Array;
    readonly layout: MltGeometryLayout;
    readonly meshes?: true;
    readonly dimension: 2 | 3;
    readonly vertices: Int32Array;
    readonly geometryOffsets?: Uint32Array;
    readonly partOffsets?: Uint32Array;
    readonly ringOffsets?: Uint32Array;
    readonly triangleOffsets?: Uint32Array;
    readonly indexBuffer?: Uint32Array;
    readonly zStep?: number;
    readonly heightMode?: HeightMode;
}

export type MltIdColumn =
    | { readonly kind: "u32"; readonly present?: Uint8Array; readonly values: Uint32Array }
    | { readonly kind: "u64"; readonly present?: Uint8Array; readonly values: BigUint64Array };

export interface MltScalarPropertyColumn<K extends string, A> {
    readonly name: string;
    readonly kind: K;
    readonly present?: Uint8Array;
    readonly values: A;
}

export type MltStringPropertyColumn =
    | { readonly name: string; readonly kind: "str"; readonly present?: Uint8Array; readonly values: readonly string[] }
    | {
          readonly name: string;
          readonly kind: "strDict";
          readonly present?: Uint8Array;
          readonly codes: Uint32Array;
          readonly dictionary: readonly string[];
      };

export type MltPropertyColumn =
    | MltScalarPropertyColumn<"bool", Uint8Array>
    | MltScalarPropertyColumn<"i8", Int8Array>
    | MltScalarPropertyColumn<"u8", Uint8Array>
    | MltScalarPropertyColumn<"i32", Int32Array>
    | MltScalarPropertyColumn<"u32", Uint32Array>
    | MltScalarPropertyColumn<"i64", BigInt64Array>
    | MltScalarPropertyColumn<"u64", BigUint64Array>
    | MltScalarPropertyColumn<"f32", Float32Array>
    | MltScalarPropertyColumn<"f64", Float64Array>
    | MltStringPropertyColumn;

export type MltMValueScope = "vertex" | "triangle";

export interface MltScalarMValueColumn<K extends string, A> {
    readonly name: string;
    readonly scope: MltMValueScope;
    readonly kind: K;
    readonly present?: Uint8Array;
    readonly offsets: Uint32Array;
    readonly values: A;
}

export type MltStringMValueColumn =
    | {
          readonly name: string;
          readonly scope: MltMValueScope;
          readonly kind: "str";
          readonly present?: Uint8Array;
          readonly offsets: Uint32Array;
          readonly values: readonly string[];
      }
    | {
          readonly name: string;
          readonly scope: MltMValueScope;
          readonly kind: "strDict";
          readonly present?: Uint8Array;
          readonly offsets: Uint32Array;
          readonly codes: Uint32Array;
          readonly dictionary: readonly string[];
      };

export type MltMValueColumn =
    | MltScalarMValueColumn<"bool", Uint8Array>
    | MltScalarMValueColumn<"i8", Int8Array>
    | MltScalarMValueColumn<"u8", Uint8Array>
    | MltScalarMValueColumn<"i32", Int32Array>
    | MltScalarMValueColumn<"u32", Uint32Array>
    | MltScalarMValueColumn<"i64", BigInt64Array>
    | MltScalarMValueColumn<"u64", BigUint64Array>
    | MltScalarMValueColumn<"f32", Float32Array>
    | MltScalarMValueColumn<"f64", Float64Array>
    | MltStringMValueColumn;

export interface MltNestedColumn {
    readonly name: string;
    readonly present?: Uint8Array;
    readonly values: readonly unknown[];
}

export interface MltColumnLayer {
    readonly name: string;
    readonly extent: number;
    readonly version: 1 | 2;
    readonly featureCount: number;
    readonly geometry: MltGeometryColumns;
    readonly ids?: MltIdColumn;
    readonly properties: readonly MltPropertyColumn[];
    readonly nested?: readonly MltNestedColumn[];
    readonly vertexValues: readonly MltMValueColumn[];
    readonly triangleValues: readonly MltMValueColumn[];
    readonly mValues: readonly MltMValueColumn[];
}

export interface MltColumnTile {
    readonly layers: readonly MltColumnLayer[];
}

export interface DecodeTileColumnsOptions {
    readonly layers?: readonly string[];
    readonly maxBytes?: number;
    readonly includeNested?: boolean;
}

export function decodeTileColumns(data: Uint8Array, options?: DecodeTileColumnsOptions): MltColumnTile {
    return { layers: decodeRawTileLayers(data, options) };
}

export type MltColumnScalar = boolean | number | bigint | string;

export function columnValue(column: MltIdColumn | MltPropertyColumn, featureIndex: number): MltColumnScalar | null {
    if (column.present !== undefined && !isBitSet(column.present, featureIndex)) return null;
    if (column.kind === "bool") return column.values[featureIndex] !== 0;
    if ("codes" in column) return column.dictionary[column.codes[featureIndex]] ?? "";
    return column.values[featureIndex];
}

export type MltMValueSlice<C extends MltMValueColumn = MltMValueColumn> = C extends { readonly kind: "strDict" }
    ? { readonly codes: Uint32Array; readonly dictionary: readonly string[] }
    : C extends { readonly kind: "str" }
      ? readonly string[]
      : C extends { readonly values: infer A }
        ? A
        : never;

export function mValueSlice<C extends MltMValueColumn>(column: C, featureIndex: number): MltMValueSlice<C> | null;
export function mValueSlice<C extends MltMValueColumn>(
    layer: MltColumnLayer,
    column: C,
    featureIndex: number,
): MltMValueSlice<C> | null;
export function mValueSlice(
    columnOrLayer: MltMValueColumn | MltColumnLayer,
    indexOrColumn: number | MltMValueColumn,
    maybeIndex?: number,
): unknown {
    const column = typeof indexOrColumn === "number" ? (columnOrLayer as MltMValueColumn) : indexOrColumn;
    const featureIndex = typeof indexOrColumn === "number" ? indexOrColumn : (maybeIndex ?? 0);
    if (column.present !== undefined && !isBitSet(column.present, featureIndex)) return null;
    const start = column.offsets[featureIndex];
    const end = column.offsets[featureIndex + 1];
    if ("codes" in column) return { codes: column.codes.subarray(start, end), dictionary: column.dictionary };
    if (column.kind === "str") return column.values.slice(start, end);
    return column.values.subarray(start, end);
}

export function toElevation(z: number, zStep: number): number {
    return z * 10 ** zStep;
}
