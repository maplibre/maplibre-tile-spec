import { type MltColumnLayer, type MltGeometryColumns, MltGeometryType } from "./columns";

/**
 * One feature's geometry as views into its layer's buffers: nothing is copied but a
 * tessellated feature's triangles. Coordinates stay as decoded, in tile coordinates with z
 * (when `dimension` is 3) on the layer's grid; `toLngLat` converts any of these vertex views.
 */
export type MltFeatureGeometry =
  | MltPointGeometry
  | MltLineGeometry
  | MltPolygonGeometry;

interface MltFeatureGeometryBase {
  readonly type: MltGeometryType;
  /** This feature's vertices, `dimension` words each. */
  readonly vertices: Int32Array;
  /**
   * Where `vertices` starts in the layer's vertex sequence, which m-value columns and other
   * per-vertex arrays run over.
   */
  readonly firstVertex: number;
}

/** A `Point` or `MultiPoint`: each vertex is a point. */
export interface MltPointGeometry extends MltFeatureGeometryBase {
  readonly kind: "point";
}

/** A `LineString` or `MultiLineString`. */
export interface MltLineGeometry extends MltFeatureGeometryBase {
  readonly kind: "line";
  readonly lines: readonly MltVertexRun[];
}

/** A `Polygon` or `MultiPolygon`. */
export interface MltPolygonGeometry extends MltFeatureGeometryBase {
  readonly kind: "polygon";
  /**
   * Each polygon's rings, ready for `earcut(vertices, holeIndices, dimension)`. Empty for a
   * `TessPolygons` layer, which stores triangles without outlines.
   */
  readonly polygons: readonly MltPolygonRings[];
  /**
   * Three indices into `vertices` per triangle, for a tessellated layer: the triangles of
   * every polygon of the feature. Left `undefined` when the layer is not tessellated.
   */
  readonly triangles: Uint32Array | undefined;
}

/** One line, or one ring, as a view of its vertices. */
export interface MltVertexRun {
  readonly vertices: Int32Array;
  /** Where `vertices` starts in the layer's vertex sequence. */
  readonly firstVertex: number;
}

export interface MltPolygonRings extends MltVertexRun {
  /** Where each hole starts in `vertices`, counted in vertices; the shell comes first. */
  readonly holeIndices: number[];
}

/** Each feature's ordinal among polygon features, which `triangleOffsets` counts, per layer. */
const polygonOrdinals = new WeakMap<MltGeometryColumns, Uint32Array>();

function polygonOrdinal(geometry: MltGeometryColumns, index: number): number {
  let ordinals = polygonOrdinals.get(geometry);
  if (!ordinals) {
    const { types } = geometry;
    ordinals = new Uint32Array(types.length);
    let polygons = 0;
    for (let f = 0; f < types.length; f++) {
      ordinals[f] = polygons;
      if (types[f] === MltGeometryType.Polygon || types[f] === MltGeometryType.MultiPolygon) {
        polygons++;
      }
    }
    polygonOrdinals.set(geometry, ordinals);
  }
  return ordinals[index];
}

/** A `TessPolygons` layer: triangles, and no outlines to walk. */
export function isTrianglesOnly(geometry: MltGeometryColumns): boolean {
  return geometry.indexBuffer !== undefined && geometry.partOffsets === undefined;
}

/** Item `i` of an offset level, or `i` itself when the layer has no such level. */
function at(level: Uint32Array | undefined, i: number): number {
  return level === undefined ? i : level[i];
}

/** Vertices `start..end` of `geometry`, as a view. */
function view(geometry: MltGeometryColumns, start: number, end: number): Int32Array {
  return geometry.vertices.subarray(start * geometry.dimension, end * geometry.dimension);
}

/** The triangle corners of polygon feature `index` as layer vertex indices, if tessellated. */
function featureCorners(geometry: MltGeometryColumns, index: number): Uint32Array | undefined {
  const { indexBuffer, triangleOffsets } = geometry;
  if (indexBuffer === undefined || triangleOffsets === undefined) return undefined;
  const ordinal = polygonOrdinal(geometry, index);
  return indexBuffer.subarray(triangleOffsets[ordinal] * 3, triangleOffsets[ordinal + 1] * 3);
}

/** `corners` counted from `v0`. Throws on a corner outside `v0..v1`. */
function rebase(corners: Uint32Array, v0: number, v1: number, index: number): Uint32Array {
  const triangles = new Uint32Array(corners.length);
  for (let i = 0; i < corners.length; i++) {
    if (corners[i] < v0 || corners[i] >= v1) {
      throw new Error(`a triangle of feature ${index} names vertex ${corners[i]}, outside ${v0}..${v1}`);
    }
    triangles[i] = corners[i] - v0;
  }
  return triangles;
}

/**
 * Feature `index` of `layer`, as views into the layer's buffers.
 *
 * Throws when `index` is not a feature of the layer.
 */
export function featureGeometry(layer: MltColumnLayer, index: number): MltFeatureGeometry {
  const { geometry } = layer;
  if (!Number.isInteger(index) || index < 0 || index >= layer.featureCount) {
    throw new RangeError(`layer "${layer.name}" has no feature ${index}`);
  }
  const type = geometry.types[index] as MltGeometryType;

  if (isTrianglesOnly(geometry)) {
    // No outlines say which vertices are whose: a feature's are the span its triangles use.
    const corners = featureCorners(geometry, index) ?? new Uint32Array(0);
    let [v0, v1] = corners.length > 0 ? [corners[0], corners[0] + 1] : [0, 0];
    for (const corner of corners) {
      if (corner < v0) v0 = corner;
      if (corner >= v1) v1 = corner + 1;
    }
    const triangles = rebase(corners, v0, v1, index);
    return { kind: "polygon", type, vertices: view(geometry, v0, v1), firstVertex: v0, polygons: [], triangles };
  }

  // Every offset level maps a run of its items to a run of the next, down to vertices.
  const { geometryOffsets: geoms, partOffsets: parts, ringOffsets: rings } = geometry;
  const [g0, g1] = [at(geoms, index), at(geoms, index + 1)];
  const [v0, v1] = [at(rings, at(parts, g0)), at(rings, at(parts, g1))];
  const vertices = view(geometry, v0, v1);

  switch (type) {
    case MltGeometryType.Point:
    case MltGeometryType.MultiPoint:
      return { kind: "point", type, vertices, firstVertex: v0 };
    case MltGeometryType.LineString:
    case MltGeometryType.MultiLineString: {
      // A line is one geometry, one part, and in a layer with rings, one ring.
      const lines: MltVertexRun[] = [];
      for (let line = g0; line < g1; line++) {
        const [a, b] = [at(rings, at(parts, line)), at(rings, at(parts, line + 1))];
        lines.push({ vertices: view(geometry, a, b), firstVertex: a });
      }
      return { kind: "line", type, vertices, firstVertex: v0, lines };
    }
    case MltGeometryType.Polygon:
    case MltGeometryType.MultiPolygon: {
      if (parts === undefined || rings === undefined) {
        throw new Error(`polygon feature ${index} of layer "${layer.name}" has no rings`);
      }
      // A polygon is one geometry, its parts are its rings.
      const polygons: MltPolygonRings[] = [];
      for (let polygon = g0; polygon < g1; polygon++) {
        const [r0, r1] = [parts[polygon], parts[polygon + 1]];
        const [a, b] = [rings[r0], rings[r1]];
        const holeIndices: number[] = [];
        for (let ring = r0 + 1; ring < r1; ring++) holeIndices.push(rings[ring] - a);
        polygons.push({ vertices: view(geometry, a, b), firstVertex: a, holeIndices });
      }
      const corners = featureCorners(geometry, index);
      const triangles = corners && rebase(corners, v0, v1, index);
      return { kind: "polygon", type, vertices, firstVertex: v0, polygons, triangles };
    }
    default:
      throw new Error(`feature ${index} of layer "${layer.name}" has unknown geometry type ${type}`);
  }
}

/**
 * The z of a vertex in metres, `-10000 + z * 10 ** zStep`, as mlt-core's `ZStep::elevation`:
 * a fine grid divides last, so `1001234` on the `-2` grid is exactly `12.34`.
 */
export function toElevation(z: number, zStep: number): number {
  return metres(z, zStep < 0, 10 ** Math.abs(zStep));
}

/** `toElevation` with the grid's `10 ** |zStep|` worked out by the caller. */
function metres(z: number, fine: boolean, power: number): number {
  return fine ? (z - 10000 * power) / power : -10000 + z * power;
}

/**
 * Convert `vertices` of `layer` (all of them, or any view `featureGeometry` returns) to
 * `longitude, latitude` in degrees, and the altitude in metres when the layer has z.
 *
 * The result has the same layout as `vertices`, so offsets, triangles and indices into one
 * index the other. `tile` is the tile's position, which only the caller knows.
 */
export function toLngLat(
  vertices: Int32Array,
  layer: MltColumnLayer,
  tile: { readonly z: number; readonly x: number; readonly y: number },
): Float64Array {
  const { dimension, zStep } = layer.geometry;
  // Web Mercator, 0 to 1 across the world, is the tile's corner plus a fraction of its width.
  const tiles = 2 ** tile.z;
  const [originX, originY] = [tile.x / tiles, tile.y / tiles];
  const scale = 1 / (layer.extent * tiles);
  const power = 10 ** Math.abs(zStep ?? 0);
  const out = new Float64Array(vertices.length);
  for (let i = 0; i < vertices.length; i += dimension) {
    const mx = originX + vertices[i] * scale;
    const my = originY + vertices[i + 1] * scale;
    out[i] = mx * 360 - 180;
    out[i + 1] = (Math.atan(Math.sinh(Math.PI * (1 - 2 * my))) * 180) / Math.PI;
    if (zStep !== undefined) out[i + 2] = metres(vertices[i + 2], zStep < 0, power);
  }
  return out;
}
