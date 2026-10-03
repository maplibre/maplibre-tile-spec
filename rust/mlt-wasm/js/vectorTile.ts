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
  /** Column names for the layer, parallel to layer_properties(). */
  layer_property_keys(layer_idx: number): string[];
  /**
   * All property values as an array of columns, parallel to layer_property_keys().
   * Each column is a typed array (numeric) or plain Array (bool/string) of
   * length feature_count. Index i gives the value for feature i; absent values
   * are NaN (numeric) or undefined (bool/string).
   */
  layer_properties(
    layer_idx: number,
  ): Array<
    | Int8Array
    | Uint8Array
    | Int32Array
    | Uint32Array
    | Float32Array
    | Float64Array
    | Array<boolean | string | undefined>
  >;
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

// ---------------------------------------------------------------------------
// loadGeometry
// ---------------------------------------------------------------------------

/** Reads vertices `start..end` into an array of `length` points; slots past them are left for the caller. */
function readPoints(
  verts: Int32Array,
  start: number,
  end: number,
  length: number,
): Point[] {
  const points: Point[] = new Array(length) as Point[];
  for (let i = start; i < end; i++) {
    points[i - start] = new Point(verts[i * 2], verts[i * 2 + 1]);
  }
  return points;
}

function lineString(verts: Int32Array, start: number, end: number): Point[] {
  return readPoints(verts, start, end, end - start);
}

/**
 * MLT stores a ring without its closing vertex; this appends it, as @mapbox/vector-tile does.
 * The closing point is a copy, so callers that mutate points in place don't move it twice.
 */
function closedRing(verts: Int32Array, start: number, end: number): Point[] {
  if (end <= start) {
    throw new Error(`polygon ring at vertex ${start} has no vertices`);
  }
  const n = end - start;
  const ring = readPoints(verts, start, end, n + 1);
  ring[n] = ring[0].clone();
  return ring;
}

function loadGeometry(
  mvtType: number,
  featureIdx: number,
  geomOffsets: Uint32Array,
  partOffsets: Uint32Array,
  ringOffsets: Uint32Array,
  verts: Int32Array,
): Point[][] {
  const hasGeomOffsets = geomOffsets.length > 0;
  const hasPartOffsets = partOffsets.length > 0;
  const hasRingOffsets = ringOffsets.length > 0;

  if (mvtType === POINT) {
    if (!hasGeomOffsets) {
      // Mixed-type layers may have part/ring indirection even for points.
      let idx = featureIdx;
      if (hasPartOffsets) idx = partOffsets[idx];
      if (hasRingOffsets) idx = ringOffsets[idx];
      return [[new Point(verts[idx * 2], verts[idx * 2 + 1])]];
    } else {
      const gStart = geomOffsets[featureIdx];
      const gEnd = geomOffsets[featureIdx + 1];
      const rings: Point[][] = new Array(gEnd - gStart) as Point[][];
      for (let g = gStart; g < gEnd; g++) {
        let idx = g;
        if (hasPartOffsets) idx = partOffsets[idx];
        if (hasRingOffsets) idx = ringOffsets[idx];
        rings[g - gStart] = [new Point(verts[idx * 2], verts[idx * 2 + 1])];
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
      return [lineString(verts, start, end)];
    } else {
      const gStart = geomOffsets[featureIdx];
      const gEnd = geomOffsets[featureIdx + 1];
      const result: Point[][] = new Array(gEnd - gStart) as Point[][];
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
        result[g - gStart] = lineString(verts, start, end);
      }
      return result;
    }
  }

  if (mvtType === POLYGON) {
    if (!hasGeomOffsets) {
      const partStart = partOffsets[featureIdx];
      const partEnd = partOffsets[featureIdx + 1];
      const rings: Point[][] = new Array(partEnd - partStart) as Point[][];
      for (let r = partStart; r < partEnd; r++) {
        rings[r - partStart] = closedRing(
          verts,
          ringOffsets[r],
          ringOffsets[r + 1],
        );
      }
      return rings;
    } else {
      // Flat ring list matching MVT convention - use loadPolygons() for grouped output.
      const gStart = geomOffsets[featureIdx];
      const gEnd = geomOffsets[featureIdx + 1];
      const result: Point[][] = [];
      for (let g = gStart; g < gEnd; g++) {
        const partStart = partOffsets[g];
        const partEnd = partOffsets[g + 1];
        for (let r = partStart; r < partEnd; r++) {
          result.push(closedRing(verts, ringOffsets[r], ringOffsets[r + 1]));
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
function loadPolygons(
  featureIdx: number,
  geomOffsets: Uint32Array,
  partOffsets: Uint32Array,
  ringOffsets: Uint32Array,
  verts: Int32Array,
): Point[][][] {
  if (geomOffsets.length === 0) {
    const partStart = partOffsets[featureIdx];
    const partEnd = partOffsets[featureIdx + 1];
    const rings: Point[][] = new Array(partEnd - partStart) as Point[][];
    for (let r = partStart; r < partEnd; r++) {
      rings[r - partStart] = closedRing(
        verts,
        ringOffsets[r],
        ringOffsets[r + 1],
      );
    }
    return [rings];
  }

  const gStart = geomOffsets[featureIdx];
  const gEnd = geomOffsets[featureIdx + 1];
  const polygons: Point[][][] = new Array(gEnd - gStart) as Point[][][];
  for (let g = gStart; g < gEnd; g++) {
    const partStart = partOffsets[g];
    const partEnd = partOffsets[g + 1];
    const rings: Point[][] = new Array(partEnd - partStart) as Point[][];
    for (let r = partStart; r < partEnd; r++) {
      rings[r - partStart] = closedRing(
        verts,
        ringOffsets[r],
        ringOffsets[r + 1],
      );
    }
    polygons[g - gStart] = rings;
  }
  return polygons;
}

// ---------------------------------------------------------------------------
// MltFeature
// ---------------------------------------------------------------------------

export class MltFeature implements VectorTileFeatureLike {
  readonly extent: number;

  private _type: 0 | 1 | 2 | 3 | undefined;
  private _id: number | undefined | null;

  constructor(
    private readonly _featureIdx: number,
    extent: number,
    private readonly _types: Uint8Array,
    private readonly _mltTypes: Uint8Array,
    private readonly _ids: Float64Array,
    private readonly _geomOffsets: Uint32Array,
    private readonly _partOffsets: Uint32Array,
    private readonly _ringOffsets: Uint32Array,
    private readonly _verts: Int32Array,
    private readonly _z: Int32Array,
    readonly zStep: number | undefined,
    private readonly propertyKeys: string[],
    private readonly propertyColumns: Array<
      | Int8Array
      | Uint8Array
      | Int32Array
      | Uint32Array
      | Float32Array
      | Float64Array
      | Array<boolean | string | undefined>
    >,
  ) {
    this.extent = extent;
    this._id = null;
  }

  get mltType(): MltGeometryType {
    return this._mltTypes[this._featureIdx] as MltGeometryType;
  }

  get type(): 0 | 1 | 2 | 3 {
    if (this._type === undefined) {
      this._type = this._types[this._featureIdx] as 0 | 1 | 2 | 3;
    }
    return this._type;
  }

  get id(): number | undefined {
    if (this._id === null) {
      const raw = this._ids[this._featureIdx];
      this._id = Number.isNaN(raw) ? undefined : raw;
    }
    return this._id as number | undefined;
  }

  get properties(): Record<string, number | string | boolean> {
    const result: Record<string, number | string | boolean> = {};
    for (let k = 0; k < this.propertyKeys.length; k++) {
      const col = this.propertyColumns[k];
      const val = col[this._featureIdx];
      if (val !== undefined) {
        result[this.propertyKeys[k]] = val as number | string | boolean;
      }
    }
    return result;
  }

  loadGeometry(): Point[][] {
    return loadGeometry(
      this.type,
      this._featureIdx,
      this._geomOffsets,
      this._partOffsets,
      this._ringOffsets,
      this._verts,
    );
  }

  /**
   * Returns one z per vertex in storage order, or an empty array when the layer has none.
   * A polygon ring's closing point is not stored, so it has no z: each ring from
   * loadGeometry() or loadPolygons() has one more point than it has z values here.
   */
  loadZ(): number[] {
    if (this._z.length === 0) return [];
    const [start, end] = vertexRange(
      this._featureIdx,
      this._geomOffsets,
      this._partOffsets,
      this._ringOffsets,
    );
    return Array.from(this._z.subarray(start, end));
  }

  /** Returns rings grouped by polygon - avoids the lossy winding-order heuristic in MVT's classifyRings. */
  loadPolygons(): Point[][][] {
    if (this.type !== POLYGON) return [this.loadGeometry()];
    return loadPolygons(
      this._featureIdx,
      this._geomOffsets,
      this._partOffsets,
      this._ringOffsets,
      this._verts,
    );
  }
}

// ---------------------------------------------------------------------------
// MltLayer
// ---------------------------------------------------------------------------

export class MltLayer implements VectorTileLayerLike {
  readonly version = 1 as const;
  readonly name: string;
  readonly extent: number;
  readonly length: number;

  private readonly _types: Uint8Array;
  private readonly _mltTypes: Uint8Array;
  private readonly _ids: Float64Array;
  private readonly _geomOffsets: Uint32Array;
  private readonly _partOffsets: Uint32Array;
  private readonly _ringOffsets: Uint32Array;
  private readonly _verts: Int32Array;
  private readonly _z: Int32Array;
  readonly zStep: number | undefined;
  readonly propertyKeys: string[];
  readonly propertyColumns: Array<
    | Int8Array
    | Uint8Array
    | Int32Array
    | Uint32Array
    | Float32Array
    | Float64Array
    | Array<boolean | string | undefined>
  >;

  constructor(
    readonly _tile: WasmMltTile,
    readonly _layerIdx: number,
    name: string,
  ) {
    this.name = name;
    this.extent = _tile.layer_extent(_layerIdx);
    this.length = _tile.feature_count(_layerIdx);
    this._types = _tile.layer_types(_layerIdx);
    this._mltTypes = _tile.layer_mlt_types(_layerIdx);
    this._ids = _tile.layer_ids(_layerIdx);
    const geom = _tile.layer_geometry(_layerIdx);
    this._geomOffsets = geom.geometry_offsets();
    this._partOffsets = geom.part_offsets();
    this._ringOffsets = geom.ring_offsets();
    this._verts = geom.vertices();
    this._z = geom.z();
    this.zStep = geom.zStep();
    this.propertyKeys = _tile.layer_property_keys(_layerIdx);
    this.propertyColumns = _tile.layer_properties(_layerIdx);
  }

  feature(i: number): MltFeature {
    return new MltFeature(
      i,
      this.extent,
      this._types,
      this._mltTypes,
      this._ids,
      this._geomOffsets,
      this._partOffsets,
      this._ringOffsets,
      this._verts,
      this._z,
      this.zStep,
      this.propertyKeys,
      this.propertyColumns,
    );
  }
}

export function decodeTile(data: Uint8Array): VectorTileLike {
  const tile = wasmDecodeTile(data) as WasmMltTile;
  const layers: Record<string, VectorTileLayerLike> = {};
  for (let i = 0; i < tile.layer_count(); i++) {
    const name = tile.layer_name(i);
    layers[name] = new MltLayer(tile, i, name);
  }
  return { layers };
}
