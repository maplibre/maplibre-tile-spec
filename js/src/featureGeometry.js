import { MltGeometryType } from "./columns.js";

const polygonOrdinals = new WeakMap();

/**
 * @param {any} geometry
 * @param {number} index
 */
function polygonOrdinal(geometry, index) {
  if (isMeshes(geometry)) {
    return index;
  }
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

/**
 * Whether `geometry` is a `TessPolygons` layer (triangles only, no outlines).
 * @param {any} geometry
 * @returns {boolean}
 */
export function isTrianglesOnly(geometry) {
  return (
    geometry.indexBuffer !== undefined &&
    geometry.partOffsets === undefined &&
    !isMeshes(geometry)
  );
}

/**
 * Whether `geometry` belongs to a v2 `Meshes` layer.
 * @param {any} geometry
 * @returns {boolean}
 */
export function isMeshes(geometry) {
  return geometry.meshes === true || geometry.types[0] === MltGeometryType.Mesh;
}

/**
 * @param {Uint32Array | undefined} level
 * @param {number} i
 */
function at(level, i) {
  return level === undefined ? i : level[i];
}

/**
 * @param {any} geometry
 * @param {number} index
 */
export function geometryStart(geometry, index) {
  return at(geometry.geometryOffsets, index);
}

/**
 * @param {any} geometry
 * @param {number} line
 */
export function lineStart(geometry, line) {
  return at(geometry.ringOffsets, at(geometry.partOffsets, line));
}

/**
 * @param {any} geometry
 * @returns {[Uint32Array, Uint32Array]}
 */
export function polygonLevels(geometry) {
  const { partOffsets, ringOffsets } = geometry;
  if (partOffsets === undefined || ringOffsets === undefined) {
    throw new Error(
      "a layer storing polygons has part and ring offsets, and this one has not",
    );
  }
  return [partOffsets, ringOffsets];
}

/**
 * @param {number} start
 * @param {number} end
 */
export function runLength(start, end) {
  if (end < start) {
    throw new Error(`a run of vertices ${start}..${end} ends before it starts`);
  }
  return end - start;
}

/**
 * @param {any} geometry
 * @param {number} start
 * @param {number} end
 */
function view(geometry, start, end) {
  runLength(start, end);
  return geometry.vertices.subarray(
    start * geometry.dimension,
    end * geometry.dimension,
  );
}

/**
 * @param {any} geometry
 * @param {number} index
 */
function featureCorners(geometry, index) {
  const { indexBuffer, triangleOffsets } = geometry;
  const ordinal = polygonOrdinal(geometry, index);
  const t0 = triangleOffsets?.[ordinal];
  const t1 = triangleOffsets?.[ordinal + 1];
  if (indexBuffer === undefined || t0 === undefined || t1 === undefined) {
    throw new Error(
      `the layer's triangles do not cover polygon ${ordinal}, feature ${index}`,
    );
  }
  return indexBuffer.subarray(t0 * 3, t1 * 3);
}

/**
 * @param {Uint32Array} corners
 * @param {number} v0
 * @param {number} v1
 * @param {number} index
 */
function rebase(corners, v0, v1, index) {
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
 * @param {number} count
 */
function identity(count) {
  const out = new Uint32Array(count + 1);
  for (let i = 0; i <= count; i++) out[i] = i;
  return out;
}

/**
 * @param {string} name
 * @param {Uint32Array} starts
 * @param {number} count
 * @param {number} [end]
 */
function checked(name, starts, count, end) {
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

const resolvedStarts = new WeakMap();

/**
 * Resolve cumulative starts of `geometry`'s features, geometries, and rings.
 * @param {any} geometry
 */
export function geometryStarts(geometry) {
  let starts = resolvedStarts.get(geometry);
  if (starts === undefined) {
    starts = resolveStarts(geometry);
    resolvedStarts.set(geometry, starts);
  }
  return starts;
}

function resolveStarts(geometry) {
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
    const geometryVertices = checked(
      "geometryVertices",
      parts,
      geometries,
      vertexCount,
    );
    return { featureGeometries, geometryVertices };
  }
  const geometryRings = checked("geometryRings", parts, geometries);
  const rings = geometryRings[geometries];
  const ringVertices = checked("ringVertices", ringOffsets, rings, vertexCount);
  const geometryVertices = new Uint32Array(geometries + 1);
  for (let g = 0; g <= geometries; g++) {
    geometryVertices[g] = lineStart(geometry, g);
  }
  return { featureGeometries, geometryVertices, geometryRings, ringVertices };
}

/**
 * Feature `index` of `layer`, as zero-copy views into the layer's buffers.
 * @param {any} layer
 * @param {number} index
 */
export function featureGeometry(layer, index) {
  const { geometry } = layer;
  if (!Number.isInteger(index) || index < 0 || index >= layer.featureCount) {
    throw new RangeError(`layer "${layer.name}" has no feature ${index}`);
  }
  const type = geometry.types[index];

  if (isTrianglesOnly(geometry)) {
    throw new Error(
      `layer "${layer.name}" is a TessPolygons layer, which stores triangles, not outlines`,
    );
  }

  const g0 = geometryStart(geometry, index);
  const g1 = geometryStart(geometry, index + 1);
  const v0 = lineStart(geometry, g0);
  const v1 = lineStart(geometry, g1);
  const vertices = view(geometry, v0, v1);

  switch (type) {
    case MltGeometryType.Point:
    case MltGeometryType.MultiPoint:
      return { kind: "point", type, vertices, firstVertex: v0 };
    case MltGeometryType.LineString:
    case MltGeometryType.MultiLineString: {
      const lines = [];
      for (let line = g0; line < g1; line++) {
        const a = lineStart(geometry, line);
        const b = lineStart(geometry, line + 1);
        lines.push({ vertices: view(geometry, a, b), firstVertex: a });
      }
      return { kind: "line", type, vertices, firstVertex: v0, lines };
    }
    case MltGeometryType.Polygon:
    case MltGeometryType.MultiPolygon: {
      const [parts, rings] = polygonLevels(geometry);
      const polygons = [];
      for (let polygon = g0; polygon < g1; polygon++) {
        const r0 = parts[polygon];
        const r1 = parts[polygon + 1];
        const a = rings[r0];
        const b = rings[r1];
        const holeIndices = [];
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
    case MltGeometryType.Mesh: {
      const triangles = rebase(featureCorners(geometry, index), v0, v1, index);
      return { kind: "mesh", type, vertices, firstVertex: v0, triangles };
    }
    default:
      throw new Error(
        `feature ${index} of layer "${layer.name}" has unknown geometry type ${type}`,
      );
  }
}
