export enum MltGeometryType {
  Point = 0,
  LineString = 1,
  Polygon = 2,
  MultiPoint = 3,
  MultiLineString = 4,
  MultiPolygon = 5,
  Mesh = 6,
}

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

export type MltColumnType = keyof MltColumnValuesByType;
export type MltColumnValues = MltColumnValuesByType[MltColumnType];

export interface MltColumn<V extends MltColumnValues = MltColumnValues> {
  readonly values: V;
  readonly present?: Uint8Array;
  readonly scope?: undefined;
}

export type MltNamedColumn = {
  [T in MltColumnType]: MltColumn<MltColumnValuesByType[T]> & {
    readonly name: string;
    readonly type: T;
  };
}[MltColumnType];

export type MltZDatum = "elevation" | "ground";

interface MltGeometryColumnsBase {
  readonly types: Uint8Array;
  readonly geometryOffsets?: Uint32Array;
  readonly partOffsets?: Uint32Array;
  readonly ringOffsets?: Uint32Array;
  readonly vertices: Int32Array;
  readonly triangleOffsets?: Uint32Array;
  readonly indexBuffer?: Uint32Array;
  readonly meshes?: true;
}

export interface MltGeometryColumns2D extends MltGeometryColumnsBase {
  readonly dimension: 2;
  readonly zStep?: undefined;
  readonly zDatum?: undefined;
}

export interface MltGeometryColumns3D extends MltGeometryColumnsBase {
  readonly dimension: 3;
  readonly zStep: number;
  readonly zDatum: MltZDatum;
}

export type MltGeometryColumns = MltGeometryColumns2D | MltGeometryColumns3D;

export type MltMValueScope = "vertex" | "triangle";

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

export interface MltColumnLayer {
  readonly name: string;
  readonly extent: number;
  readonly version: 1 | 2;
  readonly featureCount: number;
  readonly geometry: MltGeometryColumns;
  readonly ids?: MltColumn<Float64Array>;
  readonly properties: readonly MltNamedColumn[];
  readonly mValues?: readonly MltMValueColumn[];
}

export interface MltColumnTile {
  readonly layers: readonly MltColumnLayer[];
}

export interface DecodeTileColumnsOptions {
  readonly layers?: readonly string[];
}

export declare function decodeTileColumns(
  data: Uint8Array,
  options?: DecodeTileColumnsOptions,
): MltColumnTile;

export declare function columnValue<C extends MltColumn>(
  column: C,
  index: number,
): C["values"][number] | undefined;

type MltMValueSlice<C extends MltMValueColumn> = C extends {
  readonly type: "string";
}
  ? string[]
  : C extends { readonly values: infer V }
    ? V
    : never;

export declare function mValueSlice<C extends MltMValueColumn>(
  layer: MltColumnLayer,
  column: C,
  featureIndex: number,
): MltMValueSlice<C> | undefined;

export declare function toElevation(
  z: number,
  zStep: number,
  zDatum?: MltZDatum,
): number;

export interface MltVertexRun {
  readonly vertices: Int32Array;
  readonly firstVertex: number;
}

export interface MltPolygonRings extends MltVertexRun {
  readonly holeIndices: number[];
}

export interface MltPointGeometry {
  readonly kind: "point";
  readonly type: MltGeometryType;
  readonly vertices: Int32Array;
  readonly firstVertex: number;
}

export interface MltLineGeometry {
  readonly kind: "line";
  readonly type: MltGeometryType;
  readonly vertices: Int32Array;
  readonly firstVertex: number;
  readonly lines: readonly MltVertexRun[];
}

export interface MltPolygonGeometry {
  readonly kind: "polygon";
  readonly type: MltGeometryType;
  readonly vertices: Int32Array;
  readonly firstVertex: number;
  readonly polygons: readonly MltPolygonRings[];
  readonly triangles: Uint32Array | undefined;
}

export interface MltMeshGeometry {
  readonly kind: "mesh";
  readonly type: MltGeometryType;
  readonly vertices: Int32Array;
  readonly firstVertex: number;
  readonly triangles: Uint32Array;
}

export type MltFeatureGeometry =
  | MltPointGeometry
  | MltLineGeometry
  | MltPolygonGeometry
  | MltMeshGeometry;

export interface MltGeometryStarts {
  readonly featureGeometries: Uint32Array;
  readonly geometryVertices: Uint32Array;
  readonly geometryRings?: Uint32Array;
  readonly ringVertices?: Uint32Array;
}

export declare function isTrianglesOnly(geometry: MltGeometryColumns): boolean;
export declare function isMeshes(geometry: MltGeometryColumns): boolean;
export declare function geometryStarts(
  geometry: MltGeometryColumns,
): MltGeometryStarts;
export declare function featureGeometry(
  layer: MltColumnLayer,
  index: number,
): MltFeatureGeometry;

export type Position3D = [x: number, y: number, z: number];

export declare class MltPoint {
  x: number;
  y: number;
  constructor(x: number, y: number);
  clone(): MltPoint;
  equals(other: { x: number; y: number }): boolean;
}

export declare class MltFeature {
  readonly extent: number;
  readonly zStep: number | undefined;
  readonly zDatum: MltZDatum | undefined;
  get mltType(): MltGeometryType;
  get type(): 0 | 1 | 2 | 3;
  get id(): number | undefined;
  get properties(): Record<string, number | string | boolean>;
  loadGeometry(): MltPoint[][];
  loadZ(): number[];
  loadPolygons(): MltPoint[][][];
}

export declare class MltFeature3D {
  readonly extent: number;
  readonly zStep: number;
  readonly zDatum: MltZDatum;
  get mltType(): MltGeometryType;
  get type(): 0 | 1 | 2 | 3;
  get id(): number | undefined;
  get properties(): Record<string, number | string | boolean>;
  loadGeometry(): Position3D[][];
  loadPolygons(): Position3D[][][];
  loadTriangles(): Uint32Array;
  loadMValues(
    name: string,
  ): Exclude<MltColumnValues, string[]> | string[] | undefined;
}

export declare class MltLayer {
  readonly version = 1;
  readonly name: string;
  readonly extent: number;
  readonly length: number;
  readonly propertyColumns: readonly MltNamedColumn[];
  readonly propertyKeys: readonly string[];
  readonly zStep: number | undefined;
  readonly zDatum: MltZDatum | undefined;
  feature(i: number): MltFeature;
}

export declare class MltLayer3D {
  readonly name: string;
  readonly extent: number;
  readonly length: number;
  readonly propertyColumns: readonly MltNamedColumn[];
  readonly propertyKeys: readonly string[];
  readonly zStep: number;
  readonly zDatum: MltZDatum;
  readonly mValues: readonly MltMValueColumn[];
  feature(i: number): MltFeature3D;
}

export interface MltTile {
  readonly layers: Record<string, MltLayer>;
}

export interface MltTile3D {
  readonly layers: Record<string, MltLayer3D>;
}

export declare function decodeTile(data: Uint8Array): MltTile;
export declare function decodeTile3D(data: Uint8Array): MltTile3D;
export declare function tileGeoJson(data: Uint8Array): {
  type: "FeatureCollection";
  features: Array<{
    type: "Feature";
    id?: number;
    properties: Record<string, unknown>;
    geometry: { type: string; coordinates: unknown };
    triangles?: [number, number, number][];
  }>;
};
