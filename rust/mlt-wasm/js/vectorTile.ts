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
  MltGeometryType,
  type MltNamedColumn,
} from "./columns";
import {
  featureGeometry,
  isTrianglesOnly,
  type MltFeatureGeometry,
  type MltPolygonRings,
} from "./featureGeometry";

// ---------------------------------------------------------------------------
// Geometry type enums
// ---------------------------------------------------------------------------

// MVT geometry types (VectorTileFeatureLike.type)
const POINT = 1;
const LINESTRING = 2;
const POLYGON = 3;

/**
 * A vertex in 3D, as stored: `x` and `y` in tile coordinates, then `z` on the layer's
 * `zStep` grid, which is `-10000 + z * 10 ** zStep` metres.
 */
export type Position3D = [x: number, y: number, z: number];

// ---------------------------------------------------------------------------
// Geometry
// ---------------------------------------------------------------------------

/** Builds the value a geometry holds for the vertex at index `i`. */
type VertexOf<T> = (i: number) => T;

/**
 * Reads `count` vertices from `first`. With `close`, the first is appended again, as
 * @mapbox/vector-tile closes rings; it is built anew, so callers that mutate vertices in
 * place don't move it twice.
 */
function readRun<T>(vertex: VertexOf<T>, first: number, count: number, close: boolean): T[] {
  if (count === 0) return [];
  const run = new Array(close ? count + 1 : count) as T[];
  for (let k = 0; k < count; k++) run[k] = vertex(first + k);
  if (close) run[count] = vertex(first);
  return run;
}

/** A polygon's rings, each closed: the shell, then its holes. */
function polygonRings<T>(polygon: MltPolygonRings, dimension: number, vertex: VertexOf<T>): T[][] {
  const starts = [0, ...polygon.holeIndices, polygon.vertices.length / dimension];
  return starts
    .slice(1)
    .map((end, k) => readRun(vertex, polygon.firstVertex + starts[k], end - starts[k], true));
}

/**
 * A feature's geometry as @mapbox/vector-tile's rings: a ring of one per point, one per line,
 * and every polygon ring closed. A `TessPolygons` feature stores triangles without outlines,
 * so each triangle is a polygon of its own: its corners in index buffer order, closed.
 */
function featureRings<T>(g: MltFeatureGeometry, dimension: number, vertex: VertexOf<T>): T[][] {
  switch (g.kind) {
    case "point":
      return readRun(vertex, g.firstVertex, g.vertices.length / dimension, false).map((p) => [p]);
    case "line":
      return g.lines.map((line) =>
        readRun(vertex, line.firstVertex, line.vertices.length / dimension, false),
      );
    case "polygon": {
      if (g.polygons.length > 0 || g.triangles === undefined) {
        return g.polygons.flatMap((polygon) => polygonRings(polygon, dimension, vertex));
      }
      const corner = (t: number) => vertex(g.firstVertex + (g.triangles as Uint32Array)[t]);
      const rings: T[][] = [];
      for (let t = 0; t < g.triangles.length; t += 3) {
        rings.push([corner(t), corner(t + 1), corner(t + 2), corner(t)]);
      }
      return rings;
    }
  }
}

// ---------------------------------------------------------------------------
// Shared per-layer state
// ---------------------------------------------------------------------------

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
  readonly layer: MltColumnLayer;
  readonly extent: number;
  readonly mltTypes: Uint8Array;
  readonly ids: MltColumn<Float64Array> | undefined;
  readonly verts: Int32Array;
  /** Words per vertex in `verts`: `2`, or `3` when z is interleaved. */
  readonly stride: 2 | 3;
  readonly zStep: number | undefined;
  /** A `TessPolygons` layer, which has triangles and no outlines to walk. */
  readonly trianglesOnly: boolean;
  readonly properties: readonly MltNamedColumn[];
}

function readLayer(layer: MltColumnLayer): LayerData {
  const geom = layer.geometry;
  return {
    layer,
    extent: layer.extent,
    mltTypes: geom.types,
    ids: layer.ids,
    verts: geom.vertices,
    stride: geom.dimension,
    zStep: geom.zStep,
    trianglesOnly: isTrianglesOnly(geom),
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
    if (this._layer.trianglesOnly) return MltGeometryType.MultiPolygon;
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

  protected geometry(): MltFeatureGeometry {
    return featureGeometry(this._layer.layer, this._featureIdx);
  }

  protected rings(): V[][] {
    return featureRings(this.geometry(), this._layer.stride, this.vertex());
  }

  /** Rings grouped by polygon; a triangle of a `TessPolygons` feature is a polygon of its own. */
  protected polygons(): V[][][] {
    const g = this.geometry();
    if (g.kind !== "polygon" || g.polygons.length === 0) {
      const rings = featureRings(g, this._layer.stride, this.vertex());
      return g.kind === "polygon" ? rings.map((ring) => [ring]) : [rings];
    }
    const vertex = this.vertex();
    return g.polygons.map((polygon) => polygonRings(polygon, this._layer.stride, vertex));
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
    if (this._layer.stride !== 3) return [];
    const g = this.geometry();
    const z = (i: number) => g.vertices[i * 3 + 2];
    if (g.kind === "polygon" && this._layer.trianglesOnly) {
      return Array.from(g.triangles ?? [], z);
    }
    return Array.from({ length: g.vertices.length / 3 }, (_, i) => z(i));
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
      const layout = this._data.trianglesOnly
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

/**
 * Decode `data` and wrap each of its layers in `Layer`, keyed by layer name.
 *
 * Throws when two layers share a name, since a record can hold only one of them;
 * `decodeTileColumns` returns every layer.
 */
function decodeLayers<L>(
  data: Uint8Array,
  Layer: new (layer: MltColumnLayer) => L,
): Record<string, L> {
  const layers: Record<string, L> = {};
  for (const layer of decodeTileColumns(data).layers) {
    if (Object.hasOwn(layers, layer.name)) {
      throw new Error(`the tile has two layers named "${layer.name}"; decodeTileColumns keeps both`);
    }
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
