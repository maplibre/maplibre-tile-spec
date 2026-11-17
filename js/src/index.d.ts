import type {
  MltColumnLayer,
  MltColumnValues,
  MltGeometryColumns,
  MltGeometryType,
  MltMValueColumn,
  MltNamedColumn,
  MltZDatum,
} from "./columns.js";

export * from "./columns.js";

/** A contiguous run of vertices inside a layer's `vertices` buffer. */
export interface MltVertexRun {
  /** Zero-copy `Int32Array` view (`dimension` words per vertex). */
  readonly vertices: Int32Array;
  /** Index of the run's first vertex within the layer. */
  readonly firstVertex: number;
}

/** Exterior and interior rings of one polygon, ready for `earcut(vertices, holeIndices, dimension)`. */
export interface MltPolygonRings extends MltVertexRun {
  /** Start vertex index (relative to this polygon's `firstVertex`) of each interior hole ring. */
  readonly holeIndices: number[];
}

export interface MltPointGeometry {
  readonly kind: "point";
  readonly type: MltGeometryType.Point | MltGeometryType.MultiPoint;
  readonly vertices: Int32Array;
  readonly firstVertex: number;
}

export interface MltLineGeometry {
  readonly kind: "line";
  readonly type: MltGeometryType.LineString | MltGeometryType.MultiLineString;
  readonly vertices: Int32Array;
  readonly firstVertex: number;
  readonly lines: readonly MltVertexRun[];
}

export interface MltPolygonGeometry {
  readonly kind: "polygon";
  readonly type: MltGeometryType.Polygon | MltGeometryType.MultiPolygon;
  readonly vertices: Int32Array;
  readonly firstVertex: number;
  readonly polygons: readonly MltPolygonRings[];
  /** Feature-local triangle indices (`0 .. vertexCount - 1`) when pre-tessellated. */
  readonly triangles: Uint32Array | undefined;
}

export interface MltMeshGeometry {
  readonly kind: "mesh";
  readonly type: MltGeometryType.Mesh;
  readonly vertices: Int32Array;
  readonly firstVertex: number;
  /** Feature-local triangle indices (`0 .. vertexCount - 1`, `3` per triangle). */
  readonly triangles: Uint32Array;
}

/** Zero-copy view of a single feature's geometry, discriminated by `kind`. */
export type MltFeatureGeometry =
  | MltPointGeometry
  | MltLineGeometry
  | MltPolygonGeometry
  | MltMeshGeometry;

/** Resolved cumulative offset arrays across a layer's geometry hierarchy. */
export interface MltGeometryStarts {
  readonly featureGeometries: Uint32Array;
  readonly geometryVertices: Uint32Array;
  readonly geometryRings?: Uint32Array;
  readonly ringVertices?: Uint32Array;
}

/** Whether `geometry` is a `TessPolygons` layer (triangles only, no polygon outlines). */
export declare function isTrianglesOnly(geometry: MltGeometryColumns): boolean;

/** Whether `geometry` belongs to a v2 `Meshes` layer. */
export declare function isMeshes(geometry: MltGeometryColumns): boolean;

/** Resolve and memoize cumulative starts for batched GPU buffer construction. */
export declare function geometryStarts(
  geometry: MltGeometryColumns,
): MltGeometryStarts;

/** Read feature `index` of `layer` as zero-copy typed-array views. */
export declare function featureGeometry(
  layer: MltColumnLayer,
  index: number,
): MltFeatureGeometry;

/** 3D position `[x, y, z]` in integer tile coordinates (`z` on the layer's `zStep` grid). */
export type Position3D = [x: number, y: number, z: number];

/** 2D point compatible with `@mapbox/point-geometry` and MapLibre's `VectorTileFeatureLike`. */
export declare class MltPoint {
  x: number;
  y: number;
  constructor(x: number, y: number);
  clone(): MltPoint;
  equals(other: { readonly x: number; readonly y: number }): boolean;
}

/** 2D `VectorTileFeatureLike` wrapper around a decoded MLT feature. */
export declare class MltFeature {
  readonly extent: number;
  readonly zStep: number | undefined;
  readonly zDatum: MltZDatum | undefined;
  get mltType(): MltGeometryType;
  /** MVT geometry type: `1` (Point), `2` (LineString), `3` (Polygon), or `0` (Mesh). */
  get type(): 0 | 1 | 2 | 3;
  get id(): number | undefined;
  get properties(): Record<string, number | string | boolean>;
  loadGeometry(): MltPoint[][];
  loadZ(): number[];
  loadPolygons(): MltPoint[][][];
}

/** 3D feature wrapper exposing `[x, y, z]` coordinates, mesh triangles, and m-values. */
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

/** 2D `VectorTileLayerLike` wrapper around a decoded MLT layer. */
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

/** 3D layer wrapper around a decoded 3D MLT layer. */
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

/** Result of `decodeTile` (`VectorTileLike`). */
export interface MltTile {
  readonly layers: Record<string, MltLayer>;
}

/** Result of `decodeTile3D`. */
export interface MltTile3D {
  readonly layers: Record<string, MltLayer3D>;
}

/** Decode uncompressed MLT tile bytes into a MapLibre-compatible 2D `VectorTileLike` object. */
export declare function decodeTile(data: Uint8Array): MltTile;

/** Decode uncompressed 3D MLT tile bytes into `MltLayer3D` / `MltFeature3D` layers. */
export declare function decodeTile3D(data: Uint8Array): MltTile3D;

/** 2D `[x, y]` or 3D `[x, y, z]` tile-space GeoJSON coordinate. */
export type MltGeoJsonPosition = [x: number, y: number] | Position3D;

/** GeoJSON geometry produced by `tileGeoJson`. */
export type MltGeoJsonGeometry =
  | { type: "Point"; coordinates: MltGeoJsonPosition }
  | { type: "MultiPoint"; coordinates: MltGeoJsonPosition[] }
  | { type: "LineString"; coordinates: MltGeoJsonPosition[] }
  | { type: "MultiLineString"; coordinates: MltGeoJsonPosition[][] }
  | { type: "Polygon"; coordinates: MltGeoJsonPosition[][] }
  | { type: "MultiPolygon"; coordinates: MltGeoJsonPosition[][][] };

/** GeoJSON feature produced by `tileGeoJson` (matching `mlt decode --format geojson`). */
export interface MltGeoJsonFeature {
  type: "Feature";
  id?: number;
  properties: Record<string, unknown>;
  geometry: MltGeoJsonGeometry;
  triangles?: [number, number, number][];
}

/** GeoJSON `FeatureCollection` produced by `tileGeoJson`. */
export interface MltGeoJsonFeatureCollection {
  type: "FeatureCollection";
  features: MltGeoJsonFeature[];
}

/** Decode an MLT tile into a tile-coordinate GeoJSON `FeatureCollection`. */
export declare function tileGeoJson(
  data: Uint8Array,
): MltGeoJsonFeatureCollection;
