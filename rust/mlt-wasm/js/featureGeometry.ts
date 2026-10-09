import {
  type MltColumnLayer,
  type MltGeometryColumns,
  MltGeometryType,
} from "./columns";

/**
 * One feature's geometry as views into its layer's buffers: nothing is copied but a
 * tessellated feature's triangles. Coordinates stay as decoded, in tile coordinates with z
 * (when `dimension` is 3) on the layer's grid.
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
   * Where `vertices` starts in the layer's vertex sequence, which per-vertex arrays run over.
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
      if (
        types[f] === MltGeometryType.Polygon ||
        types[f] === MltGeometryType.MultiPolygon
      ) {
        polygons++;
      }
    }
    polygonOrdinals.set(geometry, ordinals);
  }
  return ordinals[index];
}

// ---------------------------------------------------------------------------
// Walking the offset levels
//
// Numbers only. `vectorTile.ts` builds arrays of its own from these and must not pay for a
// view per run; `featureGeometry` is the same walk with views on top.
// ---------------------------------------------------------------------------

/** A `TessPolygons` layer: triangles, and no outlines to walk. */
export function isTrianglesOnly(geometry: MltGeometryColumns): boolean {
  return (
    geometry.indexBuffer !== undefined && geometry.partOffsets === undefined
  );
}

/** Item `i` of an offset level, or `i` itself when the layer has no such level. */
function at(level: Uint32Array | undefined, i: number): number {
  return level === undefined ? i : level[i];
}

/** The first geometry (point, line or polygon) of feature `index`; `index + 1` gives its end. */
export function geometryStart(
  geometry: MltGeometryColumns,
  index: number,
): number {
  return at(geometry.geometryOffsets, index);
}

/**
 * The first vertex of line `line`, which is one part, and in a layer with rings, one ring.
 * Over a feature's geometry run, from `geometryStart`, it bounds the feature's vertices too.
 */
export function lineStart(geometry: MltGeometryColumns, line: number): number {
  return at(geometry.ringOffsets, at(geometry.partOffsets, line));
}

/** The levels a polygon's rings are read from: a layer storing polygons has both. */
export function polygonLevels(
  geometry: MltGeometryColumns,
): [parts: Uint32Array, rings: Uint32Array] {
  const { partOffsets, ringOffsets } = geometry;
  if (partOffsets === undefined || ringOffsets === undefined) {
    throw new Error(
      "a layer storing polygons has part and ring offsets, and this one has not",
    );
  }
  return [partOffsets, ringOffsets];
}

/** The vertices in `start..end`. Throws when an offset level runs backwards. */
export function runLength(start: number, end: number): number {
  if (end < start)
    throw new Error(`a run of vertices ${start}..${end} ends before it starts`);
  return end - start;
}

/** Vertices `start..end` of `geometry`, as a view. */
function view(
  geometry: MltGeometryColumns,
  start: number,
  end: number,
): Int32Array {
  runLength(start, end);
  return geometry.vertices.subarray(
    start * geometry.dimension,
    end * geometry.dimension,
  );
}

/**
 * The triangle corners of polygon feature `index` of a tessellated layer, as layer vertex
 * indices. Throws when the layer's `triangleOffsets` stop before the feature.
 */
export function featureCorners(
  geometry: MltGeometryColumns,
  index: number,
): Uint32Array {
  const { indexBuffer, triangleOffsets } = geometry;
  const ordinal = polygonOrdinal(geometry, index);
  const [t0, t1] = [triangleOffsets?.[ordinal], triangleOffsets?.[ordinal + 1]];
  if (indexBuffer === undefined || t0 === undefined || t1 === undefined) {
    throw new Error(
      `the layer's triangles do not cover polygon ${ordinal}, feature ${index}`,
    );
  }
  return indexBuffer.subarray(t0 * 3, t1 * 3);
}

/** `corners` counted from `v0`. Throws on a corner outside `v0..v1`. */
function rebase(
  corners: Uint32Array,
  v0: number,
  v1: number,
  index: number,
): Uint32Array {
  const triangles = new Uint32Array(corners.length);
  for (let i = 0; i < corners.length; i++) {
    if (corners[i] < v0 || corners[i] >= v1) {
      throw new Error(
        `a triangle of feature ${index} names vertex ${corners[i]}, outside ${v0}..${v1}`,
      );
    }
    triangles[i] = corners[i] - v0;
  }
  return triangles;
}

/**
 * Where each feature's geometries, each geometry's vertices and, in a layer with polygons, each
 * polygon's rings start: the layer's offset levels resolved down to its vertex sequence, for
 * building buffers for every feature at once.
 *
 * Each array has one entry more than it has items: item `i` runs from `array[i]` to
 * `array[i + 1]`. A geometry is a point, a line or a polygon; a feature has one, or several when
 * it is a multi-geometry. Every geometry kind shares one sequence, in feature order, so in a
 * layer of one kind the starts are what a renderer takes for a batch of paths or polygons.
 */
export interface MltGeometryStarts {
  /** Feature `f` has geometries `featureGeometries[f] .. featureGeometries[f + 1]`. */
  readonly featureGeometries: Uint32Array;
  /**
   * Geometry `g` has vertices `geometryVertices[g] .. geometryVertices[g + 1]`: a point's one,
   * a line's, or a polygon's, its rings one after another.
   */
  readonly geometryVertices: Uint32Array;
  /**
   * In a layer storing polygons, geometry `g` has rings `geometryRings[g] .. geometryRings[g + 1]`:
   * a polygon's shell, then its holes. A point or line of such a layer is one ring of its own.
   * Left out when the layer has no rings.
   */
  readonly geometryRings?: Uint32Array;
  /**
   * Ring `r` has vertices `ringVertices[r] .. ringVertices[r + 1]`, without the closing vertex,
   * which is not stored. Left out when the layer has no rings.
   */
  readonly ringVertices?: Uint32Array;
}

/**
 * `0 .. count`, for an offset level the layer leaves out because each item has one of the next:
 * the rule `at` applies item by item, as an array.
 */
function identity(count: number): Uint32Array {
  const out = new Uint32Array(count + 1);
  for (let i = 0; i <= count; i++) out[i] = i;
  return out;
}

/**
 * `starts`, after checking it has `count` items, starts at 0, never runs backwards, and ends at
 * `end`.
 */
function checked(
  name: string,
  starts: Uint32Array,
  count: number,
  end?: number,
): Uint32Array {
  if (starts.length !== count + 1) {
    throw new Error(
      `${name} has ${starts.length - 1} items, expected ${count}`,
    );
  }
  if (starts[0] !== 0) {
    throw new Error(`${name} starts at ${starts[0]}, expected 0`);
  }
  for (let i = 0; i < count; i++) runLength(starts[i], starts[i + 1]);
  if (end !== undefined && starts[count] !== end) {
    throw new Error(`${name} ends at ${starts[count]}, expected ${end}`);
  }
  return starts;
}

/** Each layer's starts, resolved on first use: checking and composing them is a pass over its levels. */
const resolvedStarts = new WeakMap<MltGeometryColumns, MltGeometryStarts>();

/**
 * The starts of `geometry`'s features, geometries and rings, as `MltGeometryStarts`. Stored
 * offset columns are returned as they are; levels the layer leaves out are filled in. They are
 * resolved once per layer, and every call returns the same arrays, so they must not be modified.
 *
 * Throws for a `TessPolygons` layer, which stores triangles and no runs of vertices (see
 * `indexBuffer` and `triangleOffsets`), and when the offset levels are inconsistent.
 */
export function geometryStarts(
  geometry: MltGeometryColumns,
): MltGeometryStarts {
  let starts = resolvedStarts.get(geometry);
  if (starts === undefined) {
    starts = resolveStarts(geometry);
    resolvedStarts.set(geometry, starts);
  }
  return starts;
}

function resolveStarts(geometry: MltGeometryColumns): MltGeometryStarts {
  if (isTrianglesOnly(geometry)) {
    throw new Error(
      "a TessPolygons layer stores triangles, not runs of vertices",
    );
  }
  const {
    geometryOffsets,
    partOffsets,
    ringOffsets,
    types,
    vertices,
    dimension,
  } = geometry;
  const featureGeometries = checked(
    "featureGeometries",
    geometryOffsets ?? identity(types.length),
    types.length,
  );
  if (vertices.length % dimension !== 0) {
    throw new Error(
      `the layer has ${vertices.length} vertex words, not a multiple of its dimension ${dimension}`,
    );
  }
  const vertexCount = vertices.length / dimension;
  const geometries = featureGeometries[types.length];
  const parts = partOffsets ?? identity(geometries);
  if (ringOffsets === undefined) {
    // A geometry's part is its run of vertices.
    const geometryVertices = checked(
      "geometryVertices",
      parts,
      geometries,
      vertexCount,
    );
    return { featureGeometries, geometryVertices };
  }
  // A geometry's parts are rings, and a ring's run is its vertices.
  const geometryRings = checked("geometryRings", parts, geometries);
  const rings = geometryRings[geometries];
  const ringVertices = checked("ringVertices", ringOffsets, rings, vertexCount);
  // A geometry starts at its first ring's first vertex, as featureGeometry reads it.
  const geometryVertices = new Uint32Array(geometries + 1);
  for (let g = 0; g <= geometries; g++)
    geometryVertices[g] = lineStart(geometry, g);
  return { featureGeometries, geometryVertices, geometryRings, ringVertices };
}

/**
 * Feature `index` of `layer`, as views into the layer's buffers.
 *
 * Throws when `index` is not a feature of the layer.
 */
export function featureGeometry(
  layer: MltColumnLayer,
  index: number,
): MltFeatureGeometry {
  const { geometry } = layer;
  if (!Number.isInteger(index) || index < 0 || index >= layer.featureCount) {
    throw new RangeError(`layer "${layer.name}" has no feature ${index}`);
  }
  const type = geometry.types[index] as MltGeometryType;

  if (isTrianglesOnly(geometry)) {
    // No outlines say which vertices are whose: a feature's are the span its triangles use.
    const corners = featureCorners(geometry, index);
    let [v0, v1] = corners.length > 0 ? [corners[0], corners[0] + 1] : [0, 0];
    for (const corner of corners) {
      if (corner < v0) v0 = corner;
      if (corner >= v1) v1 = corner + 1;
    }
    const triangles = rebase(corners, v0, v1, index);
    return {
      kind: "polygon",
      type,
      vertices: view(geometry, v0, v1),
      firstVertex: v0,
      polygons: [],
      triangles,
    };
  }

  // Every offset level maps a run of its items to a run of the next, down to vertices.
  const [g0, g1] = [
    geometryStart(geometry, index),
    geometryStart(geometry, index + 1),
  ];
  const [v0, v1] = [lineStart(geometry, g0), lineStart(geometry, g1)];
  const vertices = view(geometry, v0, v1);

  switch (type) {
    case MltGeometryType.Point:
    case MltGeometryType.MultiPoint:
      return { kind: "point", type, vertices, firstVertex: v0 };
    case MltGeometryType.LineString:
    case MltGeometryType.MultiLineString: {
      const lines: MltVertexRun[] = [];
      for (let line = g0; line < g1; line++) {
        const [a, b] = [
          lineStart(geometry, line),
          lineStart(geometry, line + 1),
        ];
        lines.push({ vertices: view(geometry, a, b), firstVertex: a });
      }
      return { kind: "line", type, vertices, firstVertex: v0, lines };
    }
    case MltGeometryType.Polygon:
    case MltGeometryType.MultiPolygon: {
      // A polygon is one geometry, its parts are its rings.
      const [parts, rings] = polygonLevels(geometry);
      const polygons: MltPolygonRings[] = [];
      for (let polygon = g0; polygon < g1; polygon++) {
        const [r0, r1] = [parts[polygon], parts[polygon + 1]];
        const [a, b] = [rings[r0], rings[r1]];
        const holeIndices: number[] = [];
        let previous = a;
        for (let ring = r0 + 1; ring < r1; ring++) {
          runLength(previous, rings[ring]);
          previous = rings[ring];
          holeIndices.push(previous - a);
        }
        runLength(previous, b);
        polygons.push({
          vertices: view(geometry, a, b),
          firstVertex: a,
          holeIndices,
        });
      }
      const triangles =
        geometry.indexBuffer === undefined
          ? undefined
          : rebase(featureCorners(geometry, index), v0, v1, index);
      return {
        kind: "polygon",
        type,
        vertices,
        firstVertex: v0,
        polygons,
        triangles,
      };
    }
    default:
      throw new Error(
        `feature ${index} of layer "${layer.name}" has unknown geometry type ${type}`,
      );
  }
}
