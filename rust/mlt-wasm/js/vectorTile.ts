import Point from "@mapbox/point-geometry";
import { decode_tile as wasmDecodeTile } from "../pkg/mlt_wasm.js";
import type {VectorTileFeatureLike, VectorTileLayerLike, VectorTileLike} from "@maplibre/vt-pbf";

interface WasmMltTile {
  layer_count(): number;
  layer_name(layer_idx: number): string;
  layer_extent(layer_idx: number): number;
  feature_count(layer_idx: number): number;
  feature_type(layer_idx: number, feature_idx: number): number;
  /** NaN when the feature has no ID. */
  feature_id(layer_idx: number, feature_idx: number): number;
  /**
   * Flat Int32Array: [numRings, ring0_len, x0, y0, x1, y1, …, ring1_len, …]
   * Rings are open (no repeated closing vertex).
   */
  feature_geometry(layer_idx: number, feature_idx: number): Int32Array;
  feature_properties(
    layer_idx: number,
    feature_idx: number,
  ): Record<string, number | string | boolean>;
  free(): void;
}

function decodeGeometry(raw: Int32Array): Point[][] {
  const rings: Point[][] = [];
  let i = 0;
  const numRings = raw[i++];
  for (let r = 0; r < numRings; r++) {
    const numPoints = raw[i++];
    const ring: Point[] = new Array(numPoints) as Point[];
    for (let p = 0; p < numPoints; p++) {
      ring[p] = new Point(raw[i++], raw[i++]);
    }
    rings.push(ring);
  }
  return rings;
}

class MltFeature implements VectorTileFeatureLike {
  readonly type: 0 | 1 | 2 | 3;
  readonly id: number | undefined;
  readonly extent: number;

  private _properties: Record<string, number | string | boolean> | undefined;

  constructor(
    private readonly _tile: WasmMltTile,
    private readonly _layerIdx: number,
    private readonly _featureIdx: number,
    extent: number,
  ) {
    this.extent = extent;
    this.type = _tile.feature_type(_layerIdx, _featureIdx) as 0 | 1 | 2 | 3;
    const rawId = _tile.feature_id(_layerIdx, _featureIdx);
    this.id = Number.isNaN(rawId) ? undefined : rawId;
  }

  get properties(): Record<string, number | string | boolean> {
    this._properties ??= this._tile.feature_properties(
      this._layerIdx,
      this._featureIdx,
    );
    return this._properties;
  }

  loadGeometry(): Point[][] {
    return decodeGeometry(
      this._tile.feature_geometry(this._layerIdx, this._featureIdx),
    );
  }
}

class MltLayer implements VectorTileLayerLike {
  readonly version = 1 as const;
  readonly name: string;
  readonly extent: number;
  readonly length: number;

  constructor(
    private readonly _tile: WasmMltTile,
    private readonly _layerIdx: number,
  ) {
    this.name = _tile.layer_name(_layerIdx);
    this.extent = _tile.layer_extent(_layerIdx);
    this.length = _tile.feature_count(_layerIdx);
  }

  feature(i: number): VectorTileFeatureLike {
    return new MltFeature(this._tile, this._layerIdx, i, this.extent);
  }
}

export function decodeTile(data: Uint8Array): VectorTileLike {
  const tile = wasmDecodeTile(data) as WasmMltTile;
  const layers: Record<string, VectorTileLayerLike> = {};
  for (let i = 0; i < tile.layer_count(); i++) {
    layers[tile.layer_name(i)] = new MltLayer(tile, i);
  }
  return { layers };
}
