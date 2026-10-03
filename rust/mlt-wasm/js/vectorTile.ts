import Point from "@mapbox/point-geometry";
import type {
  VectorTileFeatureLike,
  VectorTileLayerLike,
  VectorTileLike,
} from "@maplibre/vt-pbf";
import { wasmDecodeTile } from "./wasm";

// ---------------------------------------------------------------------------
// WASM interface
// ---------------------------------------------------------------------------

interface LayerGeometry {
  /** Cumulative offsets into part_offsets (multi-geometry types only). Zero-length otherwise. */
  geometry_offsets(): Uint32Array;
  /** Cumulative offsets into ring_offsets or directly into vertices. Zero-length for pure Point layers. */
  part_offsets(): Uint32Array;
  /** Cumulative vertex-count offsets. Zero-length when no ring-level indirection is needed. */
  ring_offsets(): Uint32Array;
  /** Flat [x0, y0, x1, y1, …] vertex buffer in tile coordinates. */
  vertices(): Int32Array;
  /** One z per vertex, parallel to vertices(). Zero-length when the layer has none. */
  z(): Int32Array;
  /** The z grid as the power of ten of its step in metres, or undefined when the layer has none. */
  zStep(): number | undefined;
}

interface WasmMltTile {
  layer_count(): number;
  layer_name(layer_idx: number): string;
  layer_extent(layer_idx: number): number;
  feature_count(layer_idx: number): number;
  /** Bulk MVT geometry types for the whole layer as a Uint8Array (one byte per feature: 1/2/3). */
  layer_types(layer_idx: number): Uint8Array;
  /** Original MLT geometry types (0=Point, 1=LineString, 2=Polygon, 3=MultiPoint, 4=MultiLineString, 5=MultiPolygon). */
  layer_mlt_types(layer_idx: number): Uint8Array;
  /**
   * Bulk IDs for the whole layer as a Float64Array (one f64 per feature).
   * NaN when the feature has no ID.
   */
  layer_ids(layer_idx: number): Float64Array;
  /**
   * All decoded geometry arrays for the layer in one call.
   * JS walks these directly - zero WASM calls per feature for geometry.
   */
  layer_geometry(layer_idx: number): LayerGeometry;
  /**
   * The layer's z step for decodeTile3D. Throws when the layer has no z coordinates,
   * or uses the TessPolygons or TessPolygonsWithOutlines geometry layout.
   */
  layer_z_step_3d(layer_idx: number): number;
  /** Column names for the layer, parallel to layer_properties(). */
  layer_property_keys(layer_idx: number): string[];
  /**
   * All property values as an array of columns, parallel to layer_property_keys().
   * Each column is a typed array (numeric) or plain Array (bool/string) of
   * length feature_count. Index i gives the value for feature i; absent values
   * are NaN (numeric) or undefined (bool/string).
   */
  layer_properties(layer_idx: number): PropertyColumn[];
  feature_properties(
    layer_idx: number,
    feature_idx: number,
  ): Record<string, number | string | boolean>;
  free(): void;
}

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

// ---------------------------------------------------------------------------
// Shared per-layer state
// ---------------------------------------------------------------------------

/** One decoded property column, indexed by feature. */
type PropertyColumn =
  | Int8Array
  | Uint8Array
  | Int32Array
  | Uint32Array
  | Float32Array
  | Float64Array
  | Array<boolean | string | undefined>;

/** Everything a layer's features read, fetched from WASM once per layer. */
interface LayerData {
  readonly extent: number;
  readonly types: Uint8Array;
  readonly mltTypes: Uint8Array;
  readonly ids: Float64Array;
  readonly geomOffsets: Uint32Array;
  readonly partOffsets: Uint32Array;
  readonly ringOffsets: Uint32Array;
  readonly verts: Int32Array;
  readonly z: Int32Array;
  readonly zStep: number | undefined;
  readonly propertyKeys: string[];
  readonly propertyColumns: PropertyColumn[];
}

function readLayer(tile: WasmMltTile, layerIdx: number): LayerData {
  const geom = tile.layer_geometry(layerIdx);
  return {
    extent: tile.layer_extent(layerIdx),
    types: tile.layer_types(layerIdx),
    mltTypes: tile.layer_mlt_types(layerIdx),
    ids: tile.layer_ids(layerIdx),
    geomOffsets: geom.geometry_offsets(),
    partOffsets: geom.part_offsets(),
    ringOffsets: geom.ring_offsets(),
    verts: geom.vertices(),
    z: geom.z(),
    zStep: geom.zStep(),
    propertyKeys: tile.layer_property_keys(layerIdx),
    propertyColumns: tile.layer_properties(layerIdx),
  };
}

// ---------------------------------------------------------------------------
// Features
// ---------------------------------------------------------------------------

/** What a feature is regardless of its vertices' dimension. */
abstract class FeatureBase<V> {
  readonly extent: number;

  private _type: 0 | 1 | 2 | 3 | undefined;
  private _id: number | undefined | null;

  constructor(
    protected readonly _featureIdx: number,
    protected readonly _layer: LayerData,
  ) {
    this.extent = _layer.extent;
    this._id = null;
  }

  get mltType(): MltGeometryType {
    return this._layer.mltTypes[this._featureIdx] as MltGeometryType;
  }

  get type(): 0 | 1 | 2 | 3 {
    if (this._type === undefined) {
      this._type = this._layer.types[this._featureIdx] as 0 | 1 | 2 | 3;
    }
    return this._type;
  }

  get id(): number | undefined {
    if (this._id === null) {
      const raw = this._layer.ids[this._featureIdx];
      this._id = Number.isNaN(raw) ? undefined : raw;
    }
    return this._id as number | undefined;
  }

  get properties(): Record<string, number | string | boolean> {
    const { propertyKeys, propertyColumns } = this._layer;
    const result: Record<string, number | string | boolean> = {};
    for (let k = 0; k < propertyKeys.length; k++) {
      const val = propertyColumns[k][this._featureIdx];
      if (val !== undefined) {
        result[propertyKeys[k]] = val as number | string | boolean;
      }
    }
    return result;
  }

  /** Builds the value this feature's geometry holds for vertex `i`. */
  protected abstract vertex(): VertexOf<V>;

  protected rings(): V[][] {
    const { geomOffsets, partOffsets, ringOffsets } = this._layer;
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
   */
  loadZ(): number[] {
    const { z, geomOffsets, partOffsets, ringOffsets } = this._layer;
    if (z.length === 0) return [];
    const [start, end] = vertexRange(
      this._featureIdx,
      geomOffsets,
      partOffsets,
      ringOffsets,
    );
    return Array.from(z.subarray(start, end));
  }

  /** Returns rings grouped by polygon - avoids the lossy winding-order heuristic in MVT's classifyRings. */
  loadPolygons(): Point[][][] {
    return this.polygons();
  }

  protected vertex(): VertexOf<Point> {
    const verts = this._layer.verts;
    return (i) => new Point(verts[i * 2], verts[i * 2 + 1]);
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
    const { verts, z } = this._layer;
    return (i) => [verts[i * 2], verts[i * 2 + 1], z[i]];
  }
}

// ---------------------------------------------------------------------------
// Layers
// ---------------------------------------------------------------------------

abstract class LayerBase {
  readonly extent: number;
  readonly length: number;
  readonly propertyKeys: string[];
  readonly propertyColumns: PropertyColumn[];
  protected readonly _data: LayerData;

  constructor(
    readonly _tile: WasmMltTile,
    readonly _layerIdx: number,
    readonly name: string,
  ) {
    this._data = readLayer(_tile, _layerIdx);
    this.extent = this._data.extent;
    this.length = _tile.feature_count(_layerIdx);
    this.propertyKeys = this._data.propertyKeys;
    this.propertyColumns = this._data.propertyColumns;
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
  constructor(tile: WasmMltTile, layerIdx: number, name: string) {
    super(tile, layerIdx, name);
    this.zStep = tile.layer_z_step_3d(layerIdx);
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
  Layer: new (tile: WasmMltTile, layerIdx: number, name: string) => L,
): Record<string, L> {
  const tile = wasmDecodeTile(data) as WasmMltTile;
  const layers: Record<string, L> = {};
  for (let i = 0; i < tile.layer_count(); i++) {
    const name = tile.layer_name(i);
    layers[name] = new Layer(tile, i, name);
  }
  return layers;
}

export function decodeTile(data: Uint8Array): VectorTileLike {
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
