/**
 * Geometry type codes stored per feature in an MLT layer.
 * Preserves the single-vs-multi distinction that MVT collapses.
 */
export declare enum MltGeometryType {
  Point = 0,
  LineString = 1,
  Polygon = 2,
  MultiPoint = 3,
  MultiLineString = 4,
  MultiPolygon = 5,
  /** `(x, y, z)` vertices and the triangles between them, only in a v2 `Meshes` layer. */
  Mesh = 6,
}

/**
 * Typed array / array container for each MLT scalar column type, one slot per feature.
 *
 * - `bool` is `0` / `1` in a `Uint8Array`.
 * - `i64` and `u64` are decoded into `Float64Array` (values above `Number.MAX_SAFE_INTEGER` lose precision).
 */
export interface MltColumnValuesByType {
  bool: Uint8Array;
  i8: Int8Array;
  u8: Uint8Array;
  i32: Int32Array;
  u32: Uint32Array;
  i64: Float64Array;
  u64: Float64Array;
  f32: Float32Array;
  f64: Float64Array;
  string: string[];
}

/** Scalar type name of a decoded property column. */
export type MltColumnType = keyof MltColumnValuesByType;

/** Union of all decoded column value arrays. */
export type MltColumnValues = MltColumnValuesByType[MltColumnType];

/**
 * A decoded per-feature column.
 *
 * Slots whose feature has no value hold `0` or `""`; `present` is an LSB-first bitmap
 * with one bit per feature, omitted when every feature has a value.
 */
export interface MltColumn<V extends MltColumnValues = MltColumnValues> {
  readonly values: V;
  readonly present?: Uint8Array;
  /** Always `undefined` on per-feature columns; distinguishes them from sparse `MltMValueColumn`. */
  readonly scope?: undefined;
}

/** A named property column; narrowing `type` narrows `values` to its corresponding typed array. */
export type MltNamedColumn = {
  [T in MltColumnType]: MltColumn<MltColumnValuesByType[T]> & {
    readonly name: string;
    readonly type: T;
  };
}[MltColumnType];

/**
 * What a 3D layer's `z = 0` stands for:
 * - `"absolute"`: mean sea level.
 * - `"relativeToGround"`: the terrain under each vertex; for a `Meshes` layer, the terrain at the
 *   centre of each mesh's `(x, y)` bounding box.
 */
export type MltHeightMode = "absolute" | "relativeToGround";

interface MltGeometryColumnsBase {
  /** `MltGeometryType` code for each feature (`types.length === featureCount`). */
  readonly types: Uint8Array;
  /** Cumulative geometry offsets per feature, when present. */
  readonly geometryOffsets?: Uint32Array;
  /** Cumulative part/ring offsets per geometry, when present. */
  readonly partOffsets?: Uint32Array;
  /** Cumulative vertex offsets per ring, when present. */
  readonly ringOffsets?: Uint32Array;
  /** Interleaved `(x, y)` or `(x, y, z)` integer tile coordinates (`dimension` words per vertex). */
  readonly vertices: Int32Array;
  /** Cumulative triangle counts per tessellated polygon or per `Meshes` feature. */
  readonly triangleOffsets?: Uint32Array;
  /** Flat triangle vertex indices (`3` indices per triangle), counted from the layer's first vertex. */
  readonly indexBuffer?: Uint32Array;
  /** `true` when the layer uses the v2 `Meshes` geometry layout. */
  readonly meshes?: true;
}

/** Decoded 2D geometry columns (`(x, y)` vertices). */
export interface MltGeometryColumns2D extends MltGeometryColumnsBase {
  readonly dimension: 2;
  readonly zStep?: undefined;
  readonly heightMode?: undefined;
}

/** Decoded 3D geometry columns (`(x, y, z)` vertices). */
export interface MltGeometryColumns3D extends MltGeometryColumnsBase {
  readonly dimension: 3;
  /** Power of ten of the `z` grid step in metres; convert to metres with `toElevation`. */
  readonly zStep: number;
  readonly heightMode: MltHeightMode;
}

/** Decoded geometry columns of a layer, discriminated by `dimension` (`2` or `3`). */
export type MltGeometryColumns = MltGeometryColumns2D | MltGeometryColumns3D;

/** Domain over which a v2 m-value column's values are packed. */
export type MltMValueScope = "vertex" | "triangle";

/**
 * A sparse v2 m-value column packed over the vertices or triangles of features whose `present` bit is set.
 * String m-value columns store `codes` into `dictionary`.
 */
export type MltMValueColumn =
  | {
      [T in Exclude<MltColumnType, "string">]: {
        readonly name: string;
        readonly scope: MltMValueScope;
        readonly type: T;
        readonly values: MltColumnValuesByType[T];
        readonly present?: Uint8Array;
      };
    }[Exclude<MltColumnType, "string">]
  | {
      readonly name: string;
      readonly scope: MltMValueScope;
      readonly type: "string";
      readonly codes: Uint32Array;
      readonly dictionary: readonly string[];
      readonly present?: Uint8Array;
    };

/** A single decoded MLT layer in columnar typed-array form. */
export interface MltColumnLayer {
  readonly name: string;
  readonly extent: number;
  /** Wire format version (`1` for tag `0x01`, `2` for tag `0x02`). */
  readonly version: 1 | 2;
  readonly featureCount: number;
  readonly geometry: MltGeometryColumns;
  /** Feature ID column, omitted when the layer has no IDs. */
  readonly ids?: MltColumn<Float64Array>;
  /** Scalar property columns in wire order. */
  readonly properties: readonly MltNamedColumn[];
  /** Sparse per-vertex or per-triangle m-value columns on v2 layers. */
  readonly mValues?: readonly MltMValueColumn[];
}

/** Result of `decodeTileColumns`. */
export interface MltColumnTile {
  /** Decoded layers in wire order (layer names need not be unique). */
  readonly layers: readonly MltColumnLayer[];
}

/** Options for `decodeTileColumns`. */
export interface DecodeTileColumnsOptions {
  /** Decode only layers whose name is in this list; all other layers are skipped undecoded. */
  readonly layers?: readonly string[];
  /**
   * Maximum decoded output and scratch memory budget in bytes per tile (default: `67108864` = 64 MiB,
   * matching `mlt-core::Decoder::default()`). Pass `Infinity` to disable the limit.
   */
  readonly maxBytes?: number;
}

/** Slice type returned by `mValueSlice` for a given `MltMValueColumn`. */
export type MltMValueSlice<C extends MltMValueColumn> = C extends {
  readonly type: "string";
}
  ? string[]
  : C extends { readonly values: infer V }
    ? V
    : never;

/** Decode uncompressed MLT v1/v2 tile bytes into columnar typed arrays. */
export declare function decodeTileColumns(
  data: Uint8Array,
  options?: DecodeTileColumnsOptions,
): MltColumnTile;

/** Read feature `index`'s value from `column`, or `undefined` when absent. */
export declare function columnValue<C extends MltColumn>(
  column: C,
  index: number,
): C["values"][number] | undefined;

/**
 * Read the slice of m-values owned by feature `featureIndex` in `column`, or `undefined` when absent.
 * Numeric m-value columns return a zero-copy `subarray` view; string columns return `string[]`.
 */
export declare function mValueSlice<C extends MltMValueColumn>(
  layer: MltColumnLayer,
  column: C,
  featureIndex: number,
): MltMValueSlice<C> | undefined;

/** Convert an integer `z` coordinate on a `10 ** zStep` metre grid into metres above the layer's `heightMode` base. */
export declare function toElevation(z: number, zStep: number): number;
