import { MltGeometryType, columnValue, mValueSlice } from "./columns.js";
import { decodeRawTileLayers } from "./decoder/layer.js";
import { decodeNestedColumnJson, floatToJsonValue } from "./decoder/nested.js";
import {
  geometryStart,
  isTrianglesOnly,
  lineStart,
  polygonLevels,
} from "./featureGeometry.js";

function propJsonValue(col, f) {
  const val = columnValue(col, f);
  if (val === undefined) return undefined;
  if (col.type === "bool") return val === 1;
  if (col.type === "f32") return floatToJsonValue(val, true);
  if (col.type === "f64") return floatToJsonValue(val, false);
  return val;
}

function mValueToJson(col, slice) {
  if (col.type === "string") return slice;
  if (col.type === "bool") return Array.from(slice, (v) => v === 1);
  if (col.type === "f32") return Array.from(slice, (v) => floatToJsonValue(v, true));
  if (col.type === "f64") return Array.from(slice, (v) => floatToJsonValue(v, false));
  return Array.from(slice);
}

function buildFeatureGeoJsonGeometry(layer, f) {
  const { geometry } = layer;
  const { vertices, dimension, types } = geometry;
  const hasZ = dimension === 3;

  const pos = (v) =>
    hasZ
      ? [vertices[v * 3], vertices[v * 3 + 1], vertices[v * 3 + 2]]
      : [vertices[v * 2], vertices[v * 2 + 1]];

  const closedRing = (r0, r1) => {
    if (r1 <= r0) return [];
    const ring = new Array(r1 - r0 + 1);
    for (let v = r0; v < r1; v++) ring[v - r0] = pos(v);
    ring[r1 - r0] = pos(r0);
    return ring;
  };

  if (isTrianglesOnly(geometry)) {
    const tris = geometry.triangleOffsets;
    const indices = geometry.indexBuffer;
    const lo = tris[f] * 3;
    const hi = tris[f + 1] * 3;
    const polys = [];
    for (let k = lo; k < hi; k += 3) {
      const a = pos(indices[k]);
      const b = pos(indices[k + 1]);
      const c = pos(indices[k + 2]);
      polys.push([[a, b, c, [...a]]]);
    }
    return { geometry: { type: "MultiPolygon", coordinates: polys } };
  }

  const g0 = geometryStart(geometry, f);
  const g1 = geometryStart(geometry, f + 1);
  const type = types[f];

  switch (type) {
    case MltGeometryType.Point:
      return { geometry: { type: "Point", coordinates: pos(lineStart(geometry, g0)) } };
    case MltGeometryType.MultiPoint: {
      const coords = new Array(g1 - g0);
      for (let g = g0; g < g1; g++) coords[g - g0] = pos(lineStart(geometry, g));
      return { geometry: { type: "MultiPoint", coordinates: coords } };
    }
    case MltGeometryType.LineString: {
      const v0 = lineStart(geometry, g0);
      const v1 = lineStart(geometry, g0 + 1);
      const coords = new Array(v1 - v0);
      for (let v = v0; v < v1; v++) coords[v - v0] = pos(v);
      return { geometry: { type: "LineString", coordinates: coords } };
    }
    case MltGeometryType.MultiLineString: {
      const lines = new Array(g1 - g0);
      for (let g = g0; g < g1; g++) {
        const v0 = lineStart(geometry, g);
        const v1 = lineStart(geometry, g + 1);
        const line = new Array(v1 - v0);
        for (let v = v0; v < v1; v++) line[v - v0] = pos(v);
        lines[g - g0] = line;
      }
      return { geometry: { type: "MultiLineString", coordinates: lines } };
    }
    case MltGeometryType.Polygon: {
      const [parts, rings] = polygonLevels(geometry);
      const r0 = parts[g0];
      const r1 = parts[g0 + 1];
      if (r1 <= r0) return { geometry: { type: "Polygon", coordinates: [[]] } };
      const poly = new Array(r1 - r0);
      for (let r = r0; r < r1; r++) poly[r - r0] = closedRing(rings[r], rings[r + 1]);
      return { geometry: { type: "Polygon", coordinates: poly } };
    }
    case MltGeometryType.MultiPolygon: {
      const [parts, rings] = polygonLevels(geometry);
      const polys = new Array(g1 - g0);
      for (let g = g0; g < g1; g++) {
        const r0 = parts[g];
        const r1 = parts[g + 1];
        if (r1 <= r0) {
          polys[g - g0] = [[]];
        } else {
          const poly = new Array(r1 - r0);
          for (let r = r0; r < r1; r++) poly[r - r0] = closedRing(rings[r], rings[r + 1]);
          polys[g - g0] = poly;
        }
      }
      return { geometry: { type: "MultiPolygon", coordinates: polys } };
    }
    case MltGeometryType.Mesh: {
      const coords = new Array(g1 - g0);
      for (let v = g0; v < g1; v++) coords[v - g0] = pos(v);
      const triOff = geometry.triangleOffsets;
      const indices = geometry.indexBuffer;
      const t0 = triOff[f];
      const t1 = triOff[f + 1];
      const triangles = new Array(t1 - t0);
      for (let t = t0; t < t1; t++) {
        triangles[t - t0] = [
          indices[t * 3] - g0,
          indices[t * 3 + 1] - g0,
          indices[t * 3 + 2] - g0,
        ];
      }
      return { geometry: { type: "MultiPoint", coordinates: coords }, triangles };
    }
    default:
      throw new Error(`unknown geometry type ${type}`);
  }
}

export function tileGeoJson(data) {
  const rawLayers = decodeRawTileLayers(data, { keepNested: true });
  const features = [];

  for (const layer of rawLayers) {
    const nestedCols = layer._rawNested
      ? layer._rawNested.map((n) => decodeNestedColumnJson(n, layer.featureCount))
      : [];
    for (let f = 0; f < layer.featureCount; f++) {
      const rawProps = {};
      for (const col of layer.properties) {
        const val = propJsonValue(col, f);
        if (val !== undefined) rawProps[col.name] = val;
      }
      if (layer.mValues) {
        for (const mCol of layer.mValues) {
          const slice = mValueSlice(layer, mCol, f);
          if (slice !== undefined) rawProps[`m:${mCol.name}`] = mValueToJson(mCol, slice);
        }
      }
      for (const nCol of nestedCols) {
        rawProps[nCol.name] = nCol.values[f];
      }
      if (layer.geometry.zStep !== undefined) {
        rawProps["_z_step"] = layer.geometry.zStep;
        if (layer.geometry.zDatum === "ground") rawProps["_z_datum"] = "ground";
      }
      rawProps["_layer"] = layer.name;
      rawProps["_extent"] = layer.extent;

      const properties = {};
      for (const k of Object.keys(rawProps).sort()) properties[k] = rawProps[k];

      const id = layer.ids ? columnValue(layer.ids, f) : undefined;
      const { geometry, triangles } = buildFeatureGeoJsonGeometry(layer, f);

      const feature = { type: "Feature" };
      if (id !== undefined) feature.id = id;
      feature.properties = properties;
      feature.geometry = geometry;
      if (triangles !== undefined) feature.triangles = triangles;
      features.push(feature);
    }
  }

  return { type: "FeatureCollection", features };
}
