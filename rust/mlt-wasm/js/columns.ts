import { wasmDecodeTileColumns } from "./wasm";

/** Mirrors `GeometryType` in mlt-core - preserves the single vs multi distinction that MVT collapses. */
export enum MltGeometryType {
  Point = 0,
  LineString = 1,
  Polygon = 2,
  MultiPoint = 3,
  MultiLineString = 4,
  MultiPolygon = 5,
}

/** How 64-bit integers cross the WASM boundary; the default is `number`. */
export type MltInt64Mode = "number" | "bigint";

/** The array each scalar column type comes as, one value per feature. */
interface MltColumnValuesByType<Mode extends MltInt64Mode = "number"> {
  bool: Uint8Array;
  i8: Int8Array;
  u8: Uint8Array;
  i32: Int32Array;
  u32: Uint32Array;
  i64: Mode extends "bigint" ? BigInt64Array : Float64Array;
  u64: Mode extends "bigint" ? BigUint64Array : Float64Array;
  f32: Float32Array;
  f64: Float64Array;
  string: string[];
}

/** A scalar column's physical type, also used by nested schema leaves. */
export type MltScalarType = keyof MltColumnValuesByType;

/** The type a property column was stored as. */
export type MltColumnType = MltScalarType | "nested";

/** The declared shape of a nested column, including scalar leaf types. */
export type MltNestedSchema =
  | { readonly kind: "leaf"; readonly type: MltScalarType }
  | {
      readonly kind: "struct";
      readonly fields: readonly {
        readonly name: string;
        readonly schema: MltNestedSchema;
      }[];
    }
  | { readonly kind: "list"; readonly element: MltNestedSchema }
  | { readonly kind: "map"; readonly value: MltNestedSchema };

/**
 * A nested value. Objects have no prototype; nulls remain explicit, including struct
 * fields and list elements. Only 64-bit integer leaves use bigint in exact mode.
 */
export type MltNestedValue<Mode extends MltInt64Mode = "number"> =
  | null
  | boolean
  | number
  | string
  | (Mode extends "bigint" ? bigint : never)
  | MltNestedValue<Mode>[]
  | { [key: string]: MltNestedValue<Mode> };

export type MltColumnValues<Mode extends MltInt64Mode = "number"> =
  | MltColumnValuesByType<Mode>[MltScalarType]
  | MltNestedValue<Mode>[];

/**
 * An absent scalar slot holds `0`, `0n` or `""`; `present` says which slots those are.
 * `present` is one bit per feature, LSB-first, and is left out when every feature has a value.
 * Nested columns hold their nulls directly and have no presence bitmap.
 */
export interface MltColumn<
  V extends MltColumnValues<MltInt64Mode> = MltColumnValues,
> {
  readonly values: V;
  readonly present?: Uint8Array;
}

/** A nested property column, with one value (possibly null) per feature. */
export interface MltNestedColumn<Mode extends MltInt64Mode = "number">
  extends MltColumn<MltNestedValue<Mode>[]> {
  readonly name: string;
  readonly type: "nested";
  readonly schema: MltNestedSchema;
  readonly present?: never;
}

/** A property column. Checking `type` narrows `values` to its array. */
export type MltNamedColumn<Mode extends MltInt64Mode = "number"> =
  | {
      [T in MltScalarType]: MltColumn<MltColumnValuesByType<Mode>[T]> & {
        readonly name: string;
        readonly type: T;
      };
    }[MltScalarType]
  | MltNestedColumn<Mode>;

/**
 * A layer's geometry as decoded, 2D or 3D by `dimension`.
 *
 * Offset arrays are cumulative: entry `i` starts item `i` and entry `i + 1` ends it.
 * Each level indexes into the next, and the last one present indexes vertices.
 * An array a layer does not need is left out.
 */
export type MltGeometryColumns = MltGeometryColumns2D | MltGeometryColumns3D;

interface MltGeometryColumnsBase {
  /** The MLT geometry type of each feature, see `MltGeometryType`. */
  readonly types: Uint8Array;
  readonly geometryOffsets?: Uint32Array;
  readonly partOffsets?: Uint32Array;
  readonly ringOffsets?: Uint32Array;
  /**
   * `dimension` words per vertex, in tile coordinates, interleaved as the wire stores them.
   * A polygon ring's closing vertex is not stored.
   */
  readonly vertices: Int32Array;
  /** Cumulative triangle counts per polygon feature, for a tessellated layer. */
  readonly triangleOffsets?: Uint32Array;
  /** Three vertex indices per triangle, counted from the layer's first vertex. */
  readonly indexBuffer?: Uint32Array;
}

/** `(x, y)` vertices. */
export interface MltGeometryColumns2D extends MltGeometryColumnsBase {
  readonly dimension: 2;
  readonly zStep?: undefined;
}

/** `(x, y, z)` vertices. */
export interface MltGeometryColumns3D extends MltGeometryColumnsBase {
  readonly dimension: 3;
  /** The z grid as the power of ten of its step in metres; `toElevation` gives a z in metres. */
  readonly zStep: number;
}

export interface MltColumnLayer<Mode extends MltInt64Mode = "number"> {
  readonly name: string;
  readonly extent: number;
  /** The wire version the layer was stored in. */
  readonly version: 1 | 2;
  readonly featureCount: number;
  readonly geometry: MltGeometryColumns;
  /** Left out when the layer has no id column. */
  readonly ids?: MltColumn<MltColumnValuesByType<Mode>["u64"]>;
  /**
   * Scalar columns followed by nested columns. A shared dictionary's children are named
   * by its prefix and their own names. v2 vertex-scoped (m-value) columns are not passed on.
   */
  readonly properties: readonly MltNamedColumn<Mode>[];
}

export interface MltColumnTile<Mode extends MltInt64Mode = "number"> {
  /** In wire order; layer names need not be unique. */
  readonly layers: readonly MltColumnLayer<Mode>[];
}

export interface DecodeTileColumnsOptions {
  /**
   * Decode only the layers with one of these names, every one that has it. The other
   * layers are skipped without being decoded. Every layer is decoded when left out.
   */
  readonly layers?: readonly string[];
  /**
   * `number` (default) uses Float64Array for i64/u64 columns and ids, and numbers for
   * nested integer leaves; values outside the safe integer range may lose precision.
   * `bigint` uses BigInt64Array/BigUint64Array and bigint for nested i64/u64 leaves.
   * Smaller integers and floating-point values remain numbers in both modes.
   */
  readonly int64?: MltInt64Mode;
}

/** Decode the layers of a tile into scalar arrays, nested values and integer tile geometry. */
export function decodeTileColumns(
  data: Uint8Array,
  options?: DecodeTileColumnsOptions & { int64?: "number" },
): MltColumnTile;
export function decodeTileColumns(
  data: Uint8Array,
  options: DecodeTileColumnsOptions & { int64: "bigint" },
): MltColumnTile<"bigint">;
export function decodeTileColumns(
  data: Uint8Array,
  options?: DecodeTileColumnsOptions,
): MltColumnTile<MltInt64Mode>;
export function decodeTileColumns(
  data: Uint8Array,
  options: DecodeTileColumnsOptions = {},
): MltColumnTile<MltInt64Mode> {
  // wasm-bindgen copies the array into Rust, so it never sees the caller's readonly one.
  const names = options.layers as string[] | undefined;
  return {
    layers: wasmDecodeTileColumns(
      data,
      names,
      options.int64 === "bigint",
    ) as MltColumnLayer<MltInt64Mode>[],
  };
}

/**
 * Feature `index`'s value in `column`, or `undefined` for an absent scalar value.
 * An absent nested value is `null`, as it is in the column's values array.
 *
 * Throws when `index` is not a feature, so that `undefined` only ever means no value.
 */
export function columnValue<C extends MltColumn<MltColumnValues<MltInt64Mode>>>(
  column: C,
  index: number,
): C["values"][number] | undefined {
  const { values, present } = column;
  if (!Number.isInteger(index) || index < 0 || index >= values.length) {
    throw new RangeError(`the column has no feature ${index}`);
  }
  const has =
    present === undefined || ((present[index >> 3] >> (index & 7)) & 1) === 1;
  return has ? values[index] : undefined;
}

/**
 * A z of a 3D layer in metres, `-10000 + z * 10 ** zStep`, as mlt-core's `ZStep::elevation`:
 * a fine grid divides last, so `1001234` on the `-2` grid is exactly `12.34`.
 */
export function toElevation(z: number, zStep: number): number {
  const power = 10 ** Math.abs(zStep);
  return zStep < 0 ? (z - 10000 * power) / power : -10000 + z * power;
}
