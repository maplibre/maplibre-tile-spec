import {
  MltGeometryType,
  columnValue,
  decodeTileColumns,
  mValueSlice,
} from "./columns.js";
import {
  featureGeometry,
  geometryStart,
  isMeshes,
  isTrianglesOnly,
  lineStart,
  polygonLevels,
  runLength,
} from "./featureGeometry.js";

const POINT = 1;
const LINESTRING = 2;
const POLYGON = 3;

const MVT_TYPES = [POINT, LINESTRING, POLYGON, POINT, LINESTRING, POLYGON, 0];

/**
 * Lightweight 2D point compatible with `@mapbox/point-geometry` / `VectorTileFeatureLike`.
 */
export class MltPoint {
  /**
   * @param {number} x
   * @param {number} y
   */
  constructor(x, y) {
    this.x = x;
    this.y = y;
  }

  clone() {
    return new MltPoint(this.x, this.y);
  }

  /** @param {{ x: number, y: number }} other */
  equals(other) {
    return this.x === other.x && this.y === other.y;
  }
}

/**
 * @template T
 * @param {(i: number) => T} vertex
 * @param {number} start
 * @param {number} end
 * @param {boolean} close
 * @returns {T[]}
 */
function readRun(vertex, start, end, close) {
  const count = runLength(start, end);
  if (count === 0) return [];
  const run = new Array(close ? count + 1 : count);
  for (let k = 0; k < count; k++) run[k] = vertex(start + k);
  if (close) run[count] = vertex(start);
  return run;
}

/**
 * @template T
 * @param {Uint32Array} rings
 * @param {number} r0
 * @param {number} r1
 * @param {(i: number) => T} vertex
 * @returns {T[][]}
 */
function closedRings(rings, r0, r1, vertex) {
  const out = new Array(r1 - r0);
  for (let r = r0; r < r1; r++) {
    out[r - r0] = readRun(vertex, rings[r], rings[r + 1], true);
  }
  return out;
}

function propertyValue(column, index) {
  const value = columnValue(column, index);
  return column.type === "bool" && value !== undefined ? value === 1 : value;
}

class FeatureBase {
  /**
   * @param {number} featureIdx
   * @param {{ layer: any, trianglesOnly: boolean }} layerData
   */
  constructor(featureIdx, layerData) {
    this._featureIdx = featureIdx;
    this._layer = layerData;
    this.extent = layerData.layer.extent;
  }

  get mltType() {
    return this._layer.layer.geometry.types[this._featureIdx];
  }

  get type() {
    return MVT_TYPES[this.mltType] ?? 0;
  }

  get id() {
    const { ids } = this._layer.layer;
    return ids && columnValue(ids, this._featureIdx);
  }

  get properties() {
    /** @type {Record<string, number | string | boolean>} */
    const result = {};
    for (const column of this._layer.layer.properties) {
      const value = propertyValue(column, this._featureIdx);
      if (value !== undefined) result[column.name] = value;
    }
    return result;
  }

  /** @returns {(i: number) => any} */
  _vertex() {
    throw new Error("unimplemented");
  }

  _geometries() {
    if (this._layer.trianglesOnly) {
      throw new Error(
        `layer "${this._layer.layer.name}" is a TessPolygons layer, which stores triangles, not outlines`,
      );
    }
    const { geometry } = this._layer.layer;
    return [
      geometryStart(geometry, this._featureIdx),
      geometryStart(geometry, this._featureIdx + 1),
    ];
  }

  _rings() {
    const { geometry } = this._layer.layer;
    const vertex = this._vertex();
    const [g0, g1] = this._geometries();
    if (this.mltType === MltGeometryType.Mesh) {
      return [readRun(vertex, g0, g1, false)];
    }
    switch (this.type) {
      case POINT: {
        const v0 = lineStart(geometry, g0);
        const v1 = lineStart(geometry, g1);
        const points = new Array(runLength(v0, v1));
        for (let v = v0; v < v1; v++) points[v - v0] = [vertex(v)];
        return points;
      }
      case LINESTRING: {
        const lines = new Array(g1 - g0);
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
        const [parts, rings] = polygonLevels(geometry);
        return closedRings(rings, parts[g0], parts[g1], vertex);
      }
      default:
        return [];
    }
  }

  _polygons() {
    if (this.mltType === MltGeometryType.Mesh) {
      throw new Error(
        `feature ${this._featureIdx} of layer "${this._layer.layer.name}" is a mesh; use loadGeometry() and loadTriangles() instead of loadPolygons()`,
      );
    }
    if (this.type !== POLYGON) return [this._rings()];
    const { geometry } = this._layer.layer;
    const vertex = this._vertex();
    const [g0, g1] = this._geometries();
    const [parts, rings] = polygonLevels(geometry);
    const polygons = new Array(g1 - g0);
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

export class MltFeature extends FeatureBase {
  /**
   * @param {number} featureIdx
   * @param {{ layer: any, trianglesOnly: boolean }} layerData
   */
  constructor(featureIdx, layerData) {
    super(featureIdx, layerData);
    this.zStep = layerData.layer.geometry.zStep;
    this.zDatum = layerData.layer.geometry.zDatum;
  }

  loadGeometry() {
    return this._rings();
  }

  loadZ() {
    const { geometry } = this._layer.layer;
    if (geometry.dimension !== 3) return [];
    const z = (v) => geometry.vertices[v * 3 + 2];
    const [g0, g1] = this._geometries();
    return readRun(z, lineStart(geometry, g0), lineStart(geometry, g1), false);
  }

  loadPolygons() {
    return this._polygons();
  }

  _vertex() {
    const { vertices, dimension } = this._layer.layer.geometry;
    return (i) =>
      new MltPoint(vertices[i * dimension], vertices[i * dimension + 1]);
  }
}

export class MltFeature3D extends FeatureBase {
  /**
   * @param {number} featureIdx
   * @param {{ layer: any, trianglesOnly: boolean }} layerData
   * @param {number} zStep
   * @param {"elevation" | "ground"} [zDatum="elevation"]
   */
  constructor(featureIdx, layerData, zStep, zDatum = "elevation") {
    super(featureIdx, layerData);
    this.zStep = zStep;
    this.zDatum = zDatum;
  }

  loadGeometry() {
    return this._rings();
  }

  loadPolygons() {
    return this._polygons();
  }

  loadTriangles() {
    if (this.mltType !== MltGeometryType.Mesh) {
      throw new Error(
        `feature ${this._featureIdx} of layer "${this._layer.layer.name}" is not a mesh`,
      );
    }
    const geometry = featureGeometry(this._layer.layer, this._featureIdx);
    if (geometry.kind !== "mesh") {
      throw new Error(
        `feature ${this._featureIdx} of layer "${this._layer.layer.name}" is not a mesh`,
      );
    }
    return geometry.triangles;
  }

  /** @param {string} name */
  loadMValues(name) {
    const column = this._layer.layer.mValues?.find((c) => c.name === name);
    if (column === undefined) return undefined;
    return mValueSlice(this._layer.layer, column, this._featureIdx);
  }

  _vertex() {
    const { vertices } = this._layer.layer.geometry;
    return (i) => [vertices[i * 3], vertices[i * 3 + 1], vertices[i * 3 + 2]];
  }
}

class LayerBase {
  /** @param {any} layer */
  constructor(layer) {
    this._data = { layer, trianglesOnly: isTrianglesOnly(layer.geometry) };
    this.name = layer.name;
    this.extent = layer.extent;
    this.length = layer.featureCount;
    this.propertyColumns = layer.properties;
    this.propertyKeys = layer.properties.map((column) => column.name);
  }

  /** @param {number} i */
  _featureIndex(i) {
    if (!Number.isInteger(i) || i < 0 || i >= this.length) {
      throw new RangeError(`layer "${this.name}" has no feature ${i}`);
    }
    return i;
  }
}

export class MltLayer extends LayerBase {
  /** @param {any} layer */
  constructor(layer) {
    super(layer);
    this.version = 1;
    this.zStep = this._data.layer.geometry.zStep;
    this.zDatum = this._data.layer.geometry.zDatum;
  }

  /** @param {number} i */
  feature(i) {
    return new MltFeature(this._featureIndex(i), this._data);
  }
}

export class MltLayer3D extends LayerBase {
  /** @param {any} layer */
  constructor(layer) {
    super(layer);
    const { geometry } = layer;
    if (geometry.zStep === undefined) {
      throw new Error(
        `layer "${layer.name}" has no z coordinates; decode the tile with decodeTile() instead`,
      );
    }
    if (geometry.indexBuffer !== undefined && !isMeshes(geometry)) {
      const layout = this._data.trianglesOnly
        ? "TessPolygons"
        : "TessPolygonsWithOutlines";
      throw new Error(
        `layer "${layer.name}" uses the ${layout} geometry layout, which decodeTile3D does not support`,
      );
    }
    this.zStep = geometry.zStep;
    this.zDatum = geometry.zDatum;
    this.mValues = layer.mValues ?? [];
  }

  /** @param {number} i */
  feature(i) {
    return new MltFeature3D(
      this._featureIndex(i),
      this._data,
      this.zStep,
      this.zDatum,
    );
  }
}

function decodeLayers(data, LayerClass) {
  /** @type {Record<string, any>} */
  const layers = {};
  for (const layer of decodeTileColumns(data).layers) {
    if (Object.hasOwn(layers, layer.name)) {
      throw new Error(
        `the tile has two layers named "${layer.name}"; decodeTileColumns keeps both`,
      );
    }
    layers[layer.name] = new LayerClass(layer);
  }
  return layers;
}

/**
 * Decode `data` into a MapLibre / `@mapbox/vector-tile`-compatible `{ layers }` object.
 * @param {Uint8Array} data
 */
export function decodeTile(data) {
  return { layers: decodeLayers(data, MltLayer) };
}

/**
 * Decode a 3D tile whose every layer carries z coordinates (including `Meshes` layers).
 * @param {Uint8Array} data
 */
export function decodeTile3D(data) {
  return { layers: decodeLayers(data, MltLayer3D) };
}
