import Point from "@mapbox/point-geometry";
import type {
  VectorTileFeatureLike,
  VectorTileLayerLike,
  VectorTileLike,
} from "@maplibre/vt-pbf";
import { decode_tile as wasmDecodeTile } from "../pkg/mlt_wasm.js";

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
}

interface WasmMltTile {
  layer_count(): number;
  layer_name(layer_idx: number): string;
  layer_extent(layer_idx: number): number;
  feature_count(layer_idx: number): number;
  /** Bulk geometry types for the whole layer as a Uint8Array (one byte per feature). */
  layer_types(layer_idx: number): Uint8Array;
  /**
   * Bulk IDs for the whole layer as a Float64Array (one f64 per feature).
   * NaN when the feature has no ID.
   */
  layer_ids(layer_idx: number): Float64Array;
  /**
   * All decoded geometry arrays for the layer in one call.
   * JS walks these directly — zero WASM calls per feature for geometry.
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
// Geometry constants (mirror Rust GeometryType → MVT mapping)
// ---------------------------------------------------------------------------

const POINT = 1;
const LINESTRING = 2;
const POLYGON = 3;

// ---------------------------------------------------------------------------
// loadGeometry — pure JS, zero WASM calls
//
// Faithfully replicates DecodedGeometry::to_mvt_rings from geotype.rs.
//
// Offset arrays are cumulative vertex-count arrays (not byte offsets).
// Vertex n lives at vertices[n*2], vertices[n*2+1].
// All returned rings are open (no repeated closing vertex).
// ---------------------------------------------------------------------------

/**
 * Decodes the geometry for feature `featureIdx` from the pre-fetched bulk
 * arrays. This is the JS equivalent of `DecodedGeometry::to_mvt_rings`.
 *
 * The MVT type byte (1/2/3) from `layer_types` does not distinguish
 * single-part from multi-part geometry — that distinction is implicit in
 * whether `geomOffsets` is present and in the offset ranges. We therefore
 * dispatch on both the type byte and the presence of offset arrays.
 */
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

  const openRing = (start: number, end: number): Point[] => {
    const ring: Point[] = new Array(end - start) as Point[];
    for (let i = start; i < end; i++) {
      ring[i - start] = new Point(verts[i * 2], verts[i * 2 + 1]);
    }
    return ring;
  };

  if (mvtType === POINT) {
    if (!hasGeomOffsets) {
      // Plain Point: no geometry_offsets means direct vertex indexing.
      // May have part/ring indirection if this is a mixed-type layer.
      let idx = featureIdx;
      if (hasPartOffsets) idx = partOffsets[idx];
      if (hasRingOffsets) idx = ringOffsets[idx];
      return [[new Point(verts[idx * 2], verts[idx * 2 + 1])]];
    } else {
      // MultiPoint: geometry_offsets[i..i+1] gives a range of point indices.
      // Each point may be further indirected through part/ring offsets.
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
      // Single LineString.
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
      return [openRing(start, end)];
    } else {
      // MultiLineString: geometry_offsets[i..i+1] is a range of part indices.
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
        result[g - gStart] = openRing(start, end);
      }
      return result;
    }
  }

  if (mvtType === POLYGON) {
    if (!hasGeomOffsets) {
      // Single Polygon: part_offsets[i..i+1] iterates its ring indices.
      const partStart = partOffsets[featureIdx];
      const partEnd = partOffsets[featureIdx + 1];
      const rings: Point[][] = new Array(partEnd - partStart) as Point[][];
      for (let r = partStart; r < partEnd; r++) {
        rings[r - partStart] = openRing(ringOffsets[r], ringOffsets[r + 1]);
      }
      return rings;
    } else {
      // MultiPolygon: geometry_offsets[i..i+1] iterates polygon indices;
      // each polygon's part_offsets range iterates its ring indices.
      // All rings are returned flat, matching to_mvt_rings behaviour.
      const gStart = geomOffsets[featureIdx];
      const gEnd = geomOffsets[featureIdx + 1];
      const result: Point[][] = [];
      for (let g = gStart; g < gEnd; g++) {
        const partStart = partOffsets[g];
        const partEnd = partOffsets[g + 1];
        for (let r = partStart; r < partEnd; r++) {
          result.push(openRing(ringOffsets[r], ringOffsets[r + 1]));
        }
      }
      return result;
    }
  }

  return [];
}

// ---------------------------------------------------------------------------
// MltFeature
// ---------------------------------------------------------------------------

class MltFeature implements VectorTileFeatureLike {
  readonly extent: number;

  // Lazily populated from the bulk arrays owned by MltLayer.
  // _type: undefined = not yet read.
  // _id:   null      = sentinel "not yet read"; undefined = feature has no id.
  private _type: 0 | 1 | 2 | 3 | undefined;
  private _id: number | undefined | null;

  constructor(
    private readonly _featureIdx: number,
    extent: number,
    private readonly _types: Uint8Array,
    private readonly _ids: Float64Array,
    private readonly _geomOffsets: Uint32Array,
    private readonly _partOffsets: Uint32Array,
    private readonly _ringOffsets: Uint32Array,
    private readonly _verts: Int32Array,
    private readonly _propKeys: string[],
    private readonly _propCols: Array<
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
    for (let k = 0; k < this._propKeys.length; k++) {
      const col = this._propCols[k];
      const val = col[this._featureIdx];
      if (val !== undefined) {
        result[this._propKeys[k]] = val as number | string | boolean;
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
}

// ---------------------------------------------------------------------------
// MltLayer
// ---------------------------------------------------------------------------

class MltLayer implements VectorTileLayerLike {
  readonly version = 1 as const;
  readonly name: string;
  readonly extent: number;
  readonly length: number;

  // All fetched once per layer — O(1) WASM calls regardless of feature count.
  private readonly _types: Uint8Array;
  private readonly _ids: Float64Array;
  private readonly _geomOffsets: Uint32Array;
  private readonly _partOffsets: Uint32Array;
  private readonly _ringOffsets: Uint32Array;
  private readonly _verts: Int32Array;
  private readonly _propKeys: string[];
  private readonly _propCols: Array<
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
    this._ids = _tile.layer_ids(_layerIdx);
    const geom = _tile.layer_geometry(_layerIdx);
    this._geomOffsets = geom.geometry_offsets();
    this._partOffsets = geom.part_offsets();
    this._ringOffsets = geom.ring_offsets();
    this._verts = geom.vertices();
    this._propKeys = _tile.layer_property_keys(_layerIdx);
    this._propCols = _tile.layer_properties(_layerIdx);
  }

  feature(i: number): VectorTileFeatureLike {
    return new MltFeature(
      i,
      this.extent,
      this._types,
      this._ids,
      this._geomOffsets,
      this._partOffsets,
      this._ringOffsets,
      this._verts,
      this._propKeys,
      this._propCols,
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
