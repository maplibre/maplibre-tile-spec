import Point from "@mapbox/point-geometry";
import type {
  VectorTileFeatureLike,
  VectorTileLayerLike,
  VectorTileLike,
} from "@maplibre/vt-pbf";
import {
  columnValue,
  decodeTileColumns,
  type MltColumnLayer,
  MltGeometryType,
  type MltNamedColumn,
} from "./columns";
import {
  featureCorners,
  geometryStart,
  isTrianglesOnly,
  lineStart,
  polygonLevels,
  runLength,
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
 * `zStep` grid, which `toElevation` converts to metres.
 */
export type Position3D = [x: number, y: number, z: number];

// ---------------------------------------------------------------------------
// Geometry
// ---------------------------------------------------------------------------

/** Builds the value a geometry holds for the vertex at index `i`. */
type VertexOf<T> = (i: number) => T;

/**
 * Reads vertices `start..end`. With `close`, the first is appended again, as
 * @mapbox/vector-tile closes rings; it is built anew, so callers that mutate vertices in
 * place don't move it twice.
 */
function readRun<T>(
  vertex: VertexOf<T>,
  start: number,
  end: number,
  close: boolean,
): T[] {
  const count = runLength(start, end);
  if (count === 0) return [];
  const run = new Array(close ? count + 1 : count) as T[];
  for (let k = 0; k < count; k++) run[k] = vertex(start + k);
  if (close) run[count] = vertex(start);
  return run;
}

/** Rings `r0..r1`, each closed. */
function closedRings<T>(
  rings: Uint32Array,
  r0: number,
  r1: number,
  vertex: VertexOf<T>,
): T[][] {
  const out = new Array(r1 - r0) as T[][];
  for (let r = r0; r < r1; r++)
    out[r - r0] = readRun(vertex, rings[r], rings[r + 1], true);
  return out;
}

/**
 * Each triangle as a closed ring of its corners, in index buffer order, for a `TessPolygons`
 * feature, which stores no outlines.
 */
function triangleRings<T>(corners: Uint32Array, vertex: VertexOf<T>): T[][] {
  const rings = new Array(corners.length / 3) as T[][];
  for (let t = 0; t < corners.length; t += 3) {
    rings[t / 3] = [
      vertex(corners[t]),
      vertex(corners[t + 1]),
      vertex(corners[t + 2]),
      vertex(corners[t]),
    ];
  }
  return rings;
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

/** A layer's decoded columns, and what its features read off them once per layer. */
interface LayerData {
  readonly layer: MltColumnLayer;
  /** A `TessPolygons` layer, which has triangles and no outlines to walk. */
  readonly trianglesOnly: boolean;
}

/** Reads `column` at `index` as the value a feature's `properties` holds. */
function propertyValue(
  column: MltNamedColumn,
  index: number,
): number | string | boolean | undefined {
  const value = columnValue(column, index);
  return column.type === "bool" && value !== undefined ? value === 1 : value;
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
    this.extent = _layer.layer.extent;
  }

  /**
   * The MLT geometry type. A `TessPolygons` feature is a `MultiPolygon` of its triangles,
   * whichever type it was stored as, since its outlines are not stored.
   */
  get mltType(): MltGeometryType {
    if (this._layer.trianglesOnly) return MltGeometryType.MultiPolygon;
    return this._layer.layer.geometry.types[
      this._featureIdx
    ] as MltGeometryType;
  }

  get type(): 0 | 1 | 2 | 3 {
    return MVT_TYPES[this.mltType] ?? 0;
  }

  get id(): number | undefined {
    const { ids } = this._layer.layer;
    return ids && columnValue(ids, this._featureIdx);
  }

  get properties(): Record<string, number | string | boolean> {
    const result: Record<string, number | string | boolean> = {};
    for (const column of this._layer.layer.properties) {
      const value = propertyValue(column, this._featureIdx);
      if (value !== undefined) result[column.name] = value;
    }
    return result;
  }

  /** Builds the value this feature's geometry holds for vertex `i`. */
  protected abstract vertex(): VertexOf<V>;

  /** The run of geometries (points, lines or polygons) this feature is. */
  protected geometries(): [start: number, end: number] {
    const { geometry } = this._layer.layer;
    return [
      geometryStart(geometry, this._featureIdx),
      geometryStart(geometry, this._featureIdx + 1),
    ];
  }

  /** A ring of one per point, one per line, and every polygon ring closed, as @mapbox/vector-tile. */
  protected rings(): V[][] {
    const { geometry } = this._layer.layer;
    const vertex = this.vertex();
    if (this._layer.trianglesOnly) {
      return triangleRings(featureCorners(geometry, this._featureIdx), vertex);
    }
    const [g0, g1] = this.geometries();
    switch (this.type) {
      case POINT: {
        const [v0, v1] = [lineStart(geometry, g0), lineStart(geometry, g1)];
        const points = new Array(runLength(v0, v1)) as V[][];
        for (let v = v0; v < v1; v++) points[v - v0] = [vertex(v)];
        return points;
      }
      case LINESTRING: {
        const lines = new Array(g1 - g0) as V[][];
        for (let line = g0; line < g1; line++) {
          lines[line - g0] = readRun(
            vertex,
            lineStart(geometry, line),
            lineStart(geometry, line + 1),
            false,
          );
        }
        return lines;
      }
      case POLYGON: {
        // Flat ring list matching MVT convention - use loadPolygons() for grouped output.
        const [parts, rings] = polygonLevels(geometry);
        return closedRings(rings, parts[g0], parts[g1], vertex);
      }
      default:
        return [];
    }
  }

  /** Rings grouped by polygon; a triangle of a `TessPolygons` feature is a polygon of its own. */
  protected polygons(): V[][][] {
    if (this.type !== POLYGON) return [this.rings()];
    const { geometry } = this._layer.layer;
    const vertex = this.vertex();
    if (this._layer.trianglesOnly) {
      return triangleRings(
        featureCorners(geometry, this._featureIdx),
        vertex,
      ).map((ring) => [ring]);
    }
    const [parts, rings] = polygonLevels(geometry);
    const [g0, g1] = this.geometries();
    const polygons = new Array(g1 - g0) as V[][][];
    for (let polygon = g0; polygon < g1; polygon++) {
      polygons[polygon - g0] = closedRings(
        rings,
        parts[polygon],
        parts[polygon + 1],
        vertex,
      );
    }
    return polygons;
  }
}

export class MltFeature
  extends FeatureBase<Point>
  implements VectorTileFeatureLike
{
  /** The z grid as the power of ten of its step in metres, or `undefined` when the layer has none. */
  readonly zStep: number | undefined = this._layer.layer.geometry.zStep;

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
    const { geometry } = this._layer.layer;
    if (geometry.dimension !== 3) return [];
    const z = (v: number) => geometry.vertices[v * 3 + 2];
    if (this._layer.trianglesOnly)
      return Array.from(featureCorners(geometry, this._featureIdx), z);
    const [g0, g1] = this.geometries();
    return readRun(z, lineStart(geometry, g0), lineStart(geometry, g1), false);
  }

  /** Returns rings grouped by polygon - avoids the lossy winding-order heuristic in MVT's classifyRings. */
  loadPolygons(): Point[][][] {
    return this.polygons();
  }

  protected vertex(): VertexOf<Point> {
    const { vertices, dimension } = this._layer.layer.geometry;
    return (i) =>
      new Point(vertices[i * dimension], vertices[i * dimension + 1]);
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
    const { vertices } = this._layer.layer.geometry;
    return (i) => [vertices[i * 3], vertices[i * 3 + 1], vertices[i * 3 + 2]];
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
    this._data = { layer, trianglesOnly: isTrianglesOnly(layer.geometry) };
    this.name = layer.name;
    this.extent = layer.extent;
    this.length = layer.featureCount;
    this.propertyColumns = layer.properties;
    this.propertyKeys = layer.properties.map((column) => column.name);
  }

  /** `i`, checked to be a feature of this layer, so that a bad index fails where it is given. */
  protected featureIndex(i: number): number {
    if (!Number.isInteger(i) || i < 0 || i >= this.length) {
      throw new RangeError(`layer "${this.name}" has no feature ${i}`);
    }
    return i;
  }
}

export class MltLayer extends LayerBase implements VectorTileLayerLike {
  readonly version = 1 as const;

  /** The z grid as the power of ten of its step in metres, or `undefined` when the layer has none. */
  readonly zStep: number | undefined = this._data.layer.geometry.zStep;

  feature(i: number): MltFeature {
    return new MltFeature(this.featureIndex(i), this._data);
  }
}

/** A layer with z coordinates, as `decodeTile3D` returns. */
export class MltLayer3D extends LayerBase {
  /** The z grid as the power of ten of its step in metres; `toElevation` gives a z in metres. */
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
    return new MltFeature3D(this.featureIndex(i), this._data, this.zStep);
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
      throw new Error(
        `the tile has two layers named "${layer.name}"; decodeTileColumns keeps both`,
      );
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
