import { isBitSet } from "./decoding/v2/codecs/presence";
import { decodeRawTileLayers } from "./decoding/v2/layer";

/** Geometry type codes matching `mlt-core::GeometryType` (`repr(u8)`). */
export enum MltGeometryType {
    Point = 0,
    LineString = 1,
    Polygon = 2,
    MultiPoint = 3,
    MultiLineString = 4,
    MultiPolygon = 5,
    Mesh = 6,
}

/** How a 3D layer's `z` coordinates are interpreted relative to terrain. */
export type HeightMode = "absolute" | "relativeToGround";

/** Alias for {@link HeightMode}. */
export type MltHeightMode = HeightMode;

/** What a layer's geometry and triangle columns store. */
export type MltGeometryLayout = "outlines" | "tessPolygons" | "tessPolygonsWithOutlines" | "meshes";

/** Decoded geometry columns for one MLT layer. */
export interface MltGeometryColumns {
    /** Per-feature `MltGeometryType` code (`0..=6`), length `featureCount`. */
    readonly types: Uint8Array;
    /** What `indexBuffer` and `triangleOffsets` hold. */
    readonly layout: MltGeometryLayout;
    /** Present and `true` when `layout === "meshes"`. */
    readonly meshes?: true;
    /** Words per vertex in `vertices`: `2` for `[x, y]`, `3` for `[x, y, z]`. */
    readonly dimension: 2 | 3;
    /** Flat vertex coordinates in tile units: `[x0, y0, ...]` or `[x0, y0, z0, ...]`. */
    readonly vertices: Int32Array;
    /** Cumulative offsets into `partOffsets` (or into `vertices` on a `Meshes` layer). */
    readonly geometryOffsets?: Uint32Array;
    /** Cumulative offsets into `ringOffsets` (or directly into `vertices`). */
    readonly partOffsets?: Uint32Array;
    /** Cumulative vertex-count offsets into `vertices` (counting whole vertices, not words). */
    readonly ringOffsets?: Uint32Array;
    /** Cumulative triangle counts into `indexBuffer` (multiply by `3` for index offset). */
    readonly triangleOffsets?: Uint32Array;
    /** Layer-wide vertex indices, 3 `u32`s per triangle, ready for a GPU index buffer. */
    readonly indexBuffer?: Uint32Array;
    /** Base-10 exponent (`-3..=4`) converting integer `z` values to meters (`z * 10 ** zStep`). */
    readonly zStep?: number;
    /** Height reference mode when `dimension === 3`. */
    readonly heightMode?: HeightMode;
}

export type MltIdColumn =
    | {
          readonly kind: "u32";
          readonly present?: Uint8Array;
          readonly values: Uint32Array;
      }
    | {
          readonly kind: "u64";
          readonly present?: Uint8Array;
          readonly values: BigUint64Array;
      };

export interface MltScalarPropertyColumn<K extends string, A> {
    readonly name: string;
    readonly kind: K;
    readonly present?: Uint8Array;
    readonly values: A;
}

export type MltStringPropertyColumn =
    | {
          readonly name: string;
          readonly kind: "str";
          readonly present?: Uint8Array;
          readonly values: readonly string[];
      }
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

/** Decoded v2 nested (`Struct`, `List`, or `Map`) property column. */
export interface MltNestedColumn {
    readonly name: string;
    readonly present?: Uint8Array;
    readonly values: readonly unknown[];
}

/** One decoded MLT layer in columnar TypedArray form. */
export interface MltColumnLayer {
    readonly name: string;
    readonly extent: number;
    readonly version: 1 | 2;
    readonly featureCount: number;
    readonly geometry: MltGeometryColumns;
    readonly ids?: MltIdColumn;
    readonly properties: readonly MltPropertyColumn[];
    readonly nested?: readonly MltNestedColumn[];
    /** Per-vertex m-value columns (`scope === "vertex"`). */
    readonly vertexValues: readonly MltMValueColumn[];
    /** Per-triangle m-value columns (`scope === "triangle"`, on `Meshes` layers). */
    readonly triangleValues: readonly MltMValueColumn[];
    /** All m-value columns in wire declaration order. */
    readonly mValues: readonly MltMValueColumn[];
}

export interface MltColumnTile {
    readonly layers: readonly MltColumnLayer[];
}

export interface DecodeTileColumnsOptions {
    /** Optional whitelist of layer names to decode; other layers are skipped without decoding. */
    readonly layers?: readonly string[];
    /** Maximum decoded bytes allowed before throwing `RangeError` (defaults to 64 MiB). */
    readonly maxBytes?: number;
    /** Whether to decode v2 nested (`Struct` / `List` / `Map`) columns (defaults to `true`). */
    readonly includeNested?: boolean;
}

/**
 * Decode an MLT tile (v1 `0x01` and/or v2 `0x02` layers, including 3D `Z` and `Meshes`)
 * directly into flat columnar TypedArrays.
 */
export function decodeTileColumns(data: Uint8Array, options?: DecodeTileColumnsOptions): MltColumnTile {
    return { layers: decodeRawTileLayers(data, options) };
}

export type MltColumnScalar = boolean | number | bigint | string;

/** Read a single feature's value from an ID or scalar/string property column (`null` if absent). */
export function columnValue(column: MltIdColumn | MltPropertyColumn, featureIndex: number): MltColumnScalar | null {
    if (column.present !== undefined && !isBitSet(column.present, featureIndex)) {
        return null;
    }
    if (column.kind === "bool") {
        return column.values[featureIndex] !== 0;
    }
    if ("codes" in column) {
        return column.dictionary[column.codes[featureIndex]] ?? "";
    }
    return column.values[featureIndex];
}

export type MltMValueSlice<C extends MltMValueColumn = MltMValueColumn> = C extends {
    readonly kind: "strDict";
}
    ? { readonly codes: Uint32Array; readonly dictionary: readonly string[] }
    : C extends { readonly kind: "str" }
      ? readonly string[]
      : C extends { readonly values: infer A }
        ? A
        : never;

/**
 * Return the zero-copy sub-slice of a vertex-scoped or triangle-scoped m-value column
 * belonging to `featureIndex`, or `null` when that feature has no values in `column`.
 */
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
    let column: MltMValueColumn;
    let featureIndex: number;
    if (typeof indexOrColumn === "number") {
        column = columnOrLayer as MltMValueColumn;
        featureIndex = indexOrColumn;
    } else {
        column = indexOrColumn;
        featureIndex = maybeIndex ?? 0;
    }

    if (column.present !== undefined && !isBitSet(column.present, featureIndex)) {
        return null;
    }
    const start = column.offsets[featureIndex];
    const end = column.offsets[featureIndex + 1];
    if ("codes" in column) {
        return {
            codes: column.codes.subarray(start, end),
            dictionary: column.dictionary,
        };
    }
    if (column.kind === "str") {
        return column.values.slice(start, end);
    }
    return column.values.subarray(start, end);
}

/** Convert an integer `z` coordinate to meters (`z * 10 ** zStep`). */
export function toElevation(z: number, zStep: number): number {
    return z * 10 ** zStep;
}
