import Point from "@mapbox/point-geometry";
import type {
  VectorTileFeatureLike,
  VectorTileLayerLike,
  VectorTileLike,
} from "@maplibre/vt-pbf";
import {
  decodeTileColumns,
  isPresent,
  type MltColumn,
  type MltColumnLayer,
  type MltNamedColumn,
} from "./columns";

// ---------------------------------------------------------------------------
// Geometry type enums
// ---------------------------------------------------------------------------

// MVT geometry types (VectorTileFeatureLike.type)
const POINT = 1;
const LINESTRING = 2;
const POLYGON = 3;

/** Mirrors `GeometryType` in mlt-core - preserves the single vs multi distinction that MVT collapses. */
export enum MltGeometryType {
  Point = 0,
  LineString = 1,
  Polygon = 2,
  MultiPoint = 3,
  MultiLineString = 4,
  MultiPolygon = 5,
}

/**
 * A vertex in 3D, as stored: `x` and `y` in tile coordinates, then `z` on the layer's
 * `zStep` grid, which is `-10000 + z * 10 ** zStep` metres.
 */
export type Position3D = [x: number, y: number, z: number];

// ---------------------------------------------------------------------------
// loadGeometry
// ---------------------------------------------------------------------------

/** Builds the value a geometry holds for the vertex at index `i`. */
type VertexOf<T> = (i: number) => T;

/** Reads vertices `start..end` into an array of `length` values; slots past them are left for the caller. */
function readVertices<T>(
  vertex: VertexOf<T>,
  start: number,
  end: number,
  length: number,
): T[] {
  const values: T[] = new Array(length) as T[];
  for (let i = start; i < end; i++) {
    values[i - start] = vertex(i);
  }
  return values;
}

function lineString<T>(vertex: VertexOf<T>, start: number, end: number): T[] {
  return readVertices(vertex, start, end, end - start);
}

/**
 * MLT stores a ring without its closing vertex; this appends it, as @mapbox/vector-tile does.
 * The closing vertex is built anew from the first, so callers that mutate vertices in place don't move it twice.
 */
function closedRing<T>(vertex: VertexOf<T>, start: number, end: number): T[] {
  if (end === start) return [];
  if (end < start) {
    throw new Error(`polygon ring at vertex ${start} ends before it starts`);
  }
  const n = end - start;
  const ring = readVertices(vertex, start, end, n + 1);
  ring[n] = vertex(start);
  return ring;
}

function loadGeometry<T>(
  mvtType: number,
  featureIdx: number,
  geomOffsets: Uint32Array,
  partOffsets: Uint32Array,
  ringOffsets: Uint32Array,
  vertex: VertexOf<T>,
): T[][] {
  const hasGeomOffsets = geomOffsets.length > 0;
  const hasPartOffsets = partOffsets.length > 0;
  const hasRingOffsets = ringOffsets.length > 0;

  if (mvtType === POINT) {
    if (!hasGeomOffsets) {
      // Mixed-type layers may have part/ring indirection even for points.
      let idx = featureIdx;
      if (hasPartOffsets) idx = partOffsets[idx];
      if (hasRingOffsets) idx = ringOffsets[idx];
      return [[vertex(idx)]];
    } else {
      const gStart = geomOffsets[featureIdx];
      const gEnd = geomOffsets[featureIdx + 1];
      const rings: T[][] = new Array(gEnd - gStart) as T[][];
      for (let g = gStart; g < gEnd; g++) {
        let idx = g;
        if (hasPartOffsets) idx = partOffsets[idx];
        if (hasRingOffsets) idx = ringOffsets[idx];
        rings[g - gStart] = [vertex(idx)];
      }
      return rings;
    }
  }

  if (mvtType === LINESTRING) {
    if (!hasGeomOffsets) {
      let start: number;
      let end: number;
      if (hasRingOffsets) {
        const partIdx = partOffsets[featureIdx];
        start = ringOffsets[partIdx];
        end = ringOffsets[partIdx + 1];
      } else {
        start = partOffsets[featureIdx];
        end = partOffsets[featureIdx + 1];
      }
      return [lineString(vertex, start, end)];
    } else {
      const gStart = geomOffsets[featureIdx];
      const gEnd = geomOffsets[featureIdx + 1];
      const result: T[][] = new Array(gEnd - gStart) as T[][];
      for (let g = gStart; g < gEnd; g++) {
        let start: number;
        let end: number;
        if (hasRingOffsets) {
          const partIdx = partOffsets[g];
          start = ringOffsets[partIdx];
          end = ringOffsets[partIdx + 1];
        } else {
          start = partOffsets[g];
          end = partOffsets[g + 1];
        }
        result[g - gStart] = lineString(vertex, start, end);
      }
      return result;
    }
  }

  if (mvtType === POLYGON) {
    if (!hasGeomOffsets) {
      const partStart = partOffsets[featureIdx];
      const partEnd = partOffsets[featureIdx + 1];
      const rings: T[][] = new Array(partEnd - partStart) as T[][];
      for (let r = partStart; r < partEnd; r++) {
        rings[r - partStart] = closedRing(
          vertex,
          ringOffsets[r],
          ringOffsets[r + 1],
        );
      }
      return rings;
    } else {
      // Flat ring list matching MVT convention - use loadPolygons() for grouped output.
      const gStart = geomOffsets[featureIdx];
      const gEnd = geomOffsets[featureIdx + 1];
      const result: T[][] = [];
      for (let g = gStart; g < gEnd; g++) {
        const partStart = partOffsets[g];
        const partEnd = partOffsets[g + 1];
        for (let r = partStart; r < partEnd; r++) {
          result.push(closedRing(vertex, ringOffsets[r], ringOffsets[r + 1]));
        }
      }
      return result;
    }
  }

  return [];
}

/** Returns the feature's vertex span, since every layout stores a feature's vertices contiguously. */
function vertexRange(
  featureIdx: number,
  geomOffsets: Uint32Array,
  partOffsets: Uint32Array,
  ringOffsets: Uint32Array,
): [number, number] {
  let start = featureIdx;
  let end = featureIdx + 1;
  for (const offsets of [geomOffsets, partOffsets, ringOffsets]) {
    if (offsets.length > 0) [start, end] = [offsets[start], offsets[end]];
  }
  return [start, end];
}

/** Returns rings grouped by polygon using offset arrays instead of winding-order heuristics. */
function loadPolygons<T>(
  featureIdx: number,
  geomOffsets: Uint32Array,
  partOffsets: Uint32Array,
  ringOffsets: Uint32Array,
  vertex: VertexOf<T>,
): T[][][] {
  if (geomOffsets.length === 0) {
    const partStart = partOffsets[featureIdx];
    const partEnd = partOffsets[featureIdx + 1];
    const rings: T[][] = new Array(partEnd - partStart) as T[][];
    for (let r = partStart; r < partEnd; r++) {
      rings[r - partStart] = closedRing(
        vertex,
        ringOffsets[r],
        ringOffsets[r + 1],
      );
    }
    return [rings];
  }

  const gStart = geomOffsets[featureIdx];
  const gEnd = geomOffsets[featureIdx + 1];
  const polygons: T[][][] = new Array(gEnd - gStart) as T[][][];
  for (let g = gStart; g < gEnd; g++) {
    const partStart = partOffsets[g];
    const partEnd = partOffsets[g + 1];
    const rings: T[][] = new Array(partEnd - partStart) as T[][];
    for (let r = partStart; r < partEnd; r++) {
      rings[r - partStart] = closedRing(
        vertex,
        ringOffsets[r],
        ringOffsets[r + 1],
      );
    }
    polygons[g - gStart] = rings;
  }
  return polygons;
}

/**
 * A `TessPolygons` layer stores triangles without outlines, so each triangle is a polygon of
 * its own: its three corners in index buffer order, closed by repeating the first.
 */
function triangleRings<T>(
  featureIdx: number,
  triangleOffsets: Uint32Array,
  indexBuffer: Uint32Array,
  vertex: VertexOf<T>,
): T[][] {
  const start = triangleOffsets[featureIdx];
  const end = triangleOffsets[featureIdx + 1];
  const rings: T[][] = new Array(end - start) as T[][];
  for (let t = start; t < end; t++) {
    const a = indexBuffer[t * 3];
    rings[t - start] = [
      vertex(a),
      vertex(indexBuffer[t * 3 + 1]),
      vertex(indexBuffer[t * 3 + 2]),
      vertex(a),
    ];
  }
  return rings;
}

// ---------------------------------------------------------------------------
// Shared per-layer state
// ---------------------------------------------------------------------------

const EMPTY = new Uint32Array(0);

/** MVT type (1/2/3) of each `MltGeometryType`. */
const MVT_TYPES = [
  POINT,
  LINESTRING,
  POLYGON,
  POINT,
  LINESTRING,
  POLYGON,
] as const;

/** Everything a layer's features read, taken from its decoded columns. */
interface LayerData {
  readonly extent: number;
  readonly mltTypes: Uint8Array;
  readonly ids: MltColumn<Float64Array> | undefined;
  /** Absent offset levels are zero-length, so the walkers branch on `.length`. */
  readonly geomOffsets: Uint32Array;
  readonly partOffsets: Uint32Array;
  readonly ringOffsets: Uint32Array;
  readonly verts: Int32Array;
  /** Words per vertex in `verts`: `2`, or `3` when z is interleaved. */
  readonly stride: 2 | 3;
  readonly zStep: number | undefined;
  /** Set for a `TessPolygons` layer, which has triangles and no outlines to walk. */
  readonly triangles:
    | { readonly offsets: Uint32Array; readonly indices: Uint32Array }
    | undefined;
  readonly properties: readonly MltNamedColumn[];
}

function readLayer(layer: MltColumnLayer): LayerData {
  const geom = layer.geometry;
  const { triangleOffsets, indexBuffer } = geom;
  const trianglesOnly =
    triangleOffsets !== undefined &&
    indexBuffer !== undefined &&
    geom.partOffsets === undefined;
  return {
    extent: layer.extent,
    mltTypes: geom.types,
    ids: layer.ids,
    geomOffsets: geom.geometryOffsets ?? EMPTY,
    partOffsets: geom.partOffsets ?? EMPTY,
    ringOffsets: geom.ringOffsets ?? EMPTY,
    verts: geom.vertices,
    stride: geom.dimension,
    zStep: geom.zStep,
    triangles: trianglesOnly
      ? { offsets: triangleOffsets, indices: indexBuffer }
      : undefined,
    properties: layer.properties,
  };
}

/** Reads `column` at `index` as the value a feature's `properties` holds. */
function propertyValue(
  column: MltNamedColumn,
  index: number,
): number | string | boolean | undefined {
  if (!isPresent(column, index)) return undefined;
  const value = column.values[index];
  return column.type === "bool" ? value === 1 : value;
}

// ---------------------------------------------------------------------------
// Features
// ---------------------------------------------------------------------------

/** What a feature is regardless of its vertices' dimension. */
abstract class FeatureBase<V> {
  readonly extent: number;

  constructor(
    protected readonly _featureIdx: number,
    protected readonly _layer: LayerData,
  ) {
    this.extent = _layer.extent;
  }

  /**
   * The MLT geometry type. A `TessPolygons` feature is a `MultiPolygon` of its triangles,
   * whichever type it was stored as, since its outlines are not stored.
   */
  get mltType(): MltGeometryType {
    if (this._layer.triangles !== undefined)
      return MltGeometryType.MultiPolygon;
    return this._layer.mltTypes[this._featureIdx] as MltGeometryType;
  }

  get type(): 0 | 1 | 2 | 3 {
    return MVT_TYPES[this.mltType] ?? 0;
  }

  get id(): number | undefined {
    const { ids } = this._layer;
    if (ids === undefined || !isPresent(ids, this._featureIdx))
      return undefined;
    return ids.values[this._featureIdx];
  }

  get properties(): Record<string, number | string | boolean> {
    const result: Record<string, number | string | boolean> = {};
    for (const column of this._layer.properties) {
      const value = propertyValue(column, this._featureIdx);
      if (value !== undefined) result[column.name] = value;
    }
    return result;
  }

  /** Builds the value this feature's geometry holds for vertex `i`. */
  protected abstract vertex(): VertexOf<V>;

  protected rings(): V[][] {
    const { geomOffsets, partOffsets, ringOffsets, triangles } = this._layer;
    if (triangles !== undefined) {
      return triangleRings(
        this._featureIdx,
        triangles.offsets,
        triangles.indices,
        this.vertex(),
      );
    }
    return loadGeometry(
      this.type,
      this._featureIdx,
      geomOffsets,
      partOffsets,
      ringOffsets,
      this.vertex(),
    );
  }

  protected polygons(): V[][][] {
    if (this._layer.triangles !== undefined) {
      return this.rings().map((ring) => [ring]);
    }
    if (this.type !== POLYGON) return [this.rings()];
    const { geomOffsets, partOffsets, ringOffsets } = this._layer;
    return loadPolygons(
      this._featureIdx,
      geomOffsets,
      partOffsets,
      ringOffsets,
      this.vertex(),
    );
  }
}

export class MltFeature
  extends FeatureBase<Point>
  implements VectorTileFeatureLike
{
  /** The z grid as the power of ten of its step in metres, or `undefined` when the layer has none. */
  readonly zStep: number | undefined = this._layer.zStep;

  loadGeometry(): Point[][] {
    return this.rings();
  }

  /**
   * Returns one z per vertex in storage order, or an empty array when the layer has none.
   * A polygon ring's closing point is not stored, so it has no z: each ring from
   * loadGeometry() or loadPolygons() has one more point than it has z values here.
   * A `TessPolygons` feature has the z of each triangle corner, in index buffer order.
   */
  loadZ(): number[] {
    const { verts, stride, geomOffsets, partOffsets, ringOffsets, triangles } =
      this._layer;
    if (stride !== 3) return [];
    const z = (i: number) => verts[i * 3 + 2];
    if (triangles !== undefined) {
      const start = triangles.offsets[this._featureIdx] * 3;
      const end = triangles.offsets[this._featureIdx + 1] * 3;
      return Array.from(triangles.indices.subarray(start, end), z);
    }
    const [start, end] = vertexRange(
      this._featureIdx,
      geomOffsets,
      partOffsets,
      ringOffsets,
    );
    const values = new Array<number>(end - start);
    for (let i = start; i < end; i++) values[i - start] = z(i);
    return values;
  }

  /** Returns rings grouped by polygon - avoids the lossy winding-order heuristic in MVT's classifyRings. */
  loadPolygons(): Point[][][] {
    return this.polygons();
  }

  protected vertex(): VertexOf<Point> {
    const { verts, stride } = this._layer;
    return (i) => new Point(verts[i * stride], verts[i * stride + 1]);
  }
}

/** A feature of a layer with z coordinates, as `decodeTile3D` returns. */
export class MltFeature3D extends FeatureBase<Position3D> {
  constructor(
    featureIdx: number,
    layer: LayerData,
    readonly zStep: number,
  ) {
    super(featureIdx, layer);
  }

  /** Like `MltFeature.loadGeometry`, each vertex `[x, y, z]` with `z` on the `zStep` grid. */
  loadGeometry(): Position3D[][] {
    return this.rings();
  }

  /** Like `MltFeature.loadPolygons`, each vertex `[x, y, z]` with `z` on the `zStep` grid. */
  loadPolygons(): Position3D[][][] {
    return this.polygons();
  }

  protected vertex(): VertexOf<Position3D> {
    const { verts } = this._layer;
    return (i) => [verts[i * 3], verts[i * 3 + 1], verts[i * 3 + 2]];
  }
}

// ---------------------------------------------------------------------------
// Layers
// ---------------------------------------------------------------------------

abstract class LayerBase {
  readonly name: string;
  readonly extent: number;
  readonly length: number;
  /** The property columns as decoded, see `decodeTileColumns`. */
  readonly propertyColumns: readonly MltNamedColumn[];
  /** The property column names, parallel to `propertyColumns`. */
  readonly propertyKeys: readonly string[];
  protected readonly _data: LayerData;

  constructor(layer: MltColumnLayer) {
    this._data = readLayer(layer);
    this.name = layer.name;
    this.extent = layer.extent;
    this.length = layer.featureCount;
    this.propertyColumns = layer.properties;
    this.propertyKeys = layer.properties.map((column) => column.name);
  }
}

export class MltLayer extends LayerBase implements VectorTileLayerLike {
  readonly version = 1 as const;

  /** The z grid as the power of ten of its step in metres, or `undefined` when the layer has none. */
  readonly zStep: number | undefined = this._data.zStep;

  feature(i: number): MltFeature {
    return new MltFeature(i, this._data);
  }
}

/** A layer with z coordinates, as `decodeTile3D` returns. */
export class MltLayer3D extends LayerBase {
  /** The z grid as the power of ten of its step in metres: a raw z is `-10000 + z * 10 ** zStep` metres. */
  readonly zStep: number;

  /**
   * Throws when the layer has no z coordinates, or uses the `TessPolygons` or
   * `TessPolygonsWithOutlines` geometry layout, which `decodeTile3D` does not support.
   */
  constructor(layer: MltColumnLayer) {
    super(layer);
    const { zStep, indexBuffer } = layer.geometry;
    if (zStep === undefined) {
      throw new Error(
        `layer "${layer.name}" has no z coordinates; decode the tile with decodeTile() instead`,
      );
    }
    if (indexBuffer !== undefined) {
      const layout = this._data.triangles
        ? "TessPolygons"
        : "TessPolygonsWithOutlines";
      throw new Error(
        `layer "${layer.name}" uses the ${layout} geometry layout, which decodeTile3D does not support`,
      );
    }
    this.zStep = zStep;
  }

  feature(i: number): MltFeature3D {
    return new MltFeature3D(i, this._data, this.zStep);
  }
}

// ---------------------------------------------------------------------------
// Entry points
// ---------------------------------------------------------------------------

/** Decode `data` and wrap each of its layers in `Layer`, keyed by layer name. */
function decodeLayers<L>(
  data: Uint8Array,
  Layer: new (layer: MltColumnLayer) => L,
): Record<string, L> {
  const layers: Record<string, L> = {};
  for (const layer of decodeTileColumns(data).layers) {
    layers[layer.name] = new Layer(layer);
  }
  return layers;
}

export interface MltTile extends VectorTileLike {
  readonly layers: Record<string, MltLayer>;
}

export function decodeTile(data: Uint8Array): MltTile {
  return { layers: decodeLayers(data, MltLayer) };
}

export interface MltTile3D {
  readonly layers: Record<string, MltLayer3D>;
}

/**
 * Decode a tile whose every layer has z coordinates, so every vertex is `[x, y, z]`.
 *
 * Throws when any layer has none, for which `decodeTile` reads the tile in 2D, or when any
 * layer uses the `TessPolygons` or `TessPolygonsWithOutlines` geometry layout.
 */
export function decodeTile3D(data: Uint8Array): MltTile3D {
  return { layers: decodeLayers(data, MltLayer3D) };
}
