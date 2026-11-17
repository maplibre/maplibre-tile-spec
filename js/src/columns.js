import { decodeRawTileLayers } from "./decoder/layer.js";

export const MltGeometryType = Object.freeze({
  Point: 0,
  LineString: 1,
  Polygon: 2,
  MultiPoint: 3,
  MultiLineString: 4,
  MultiPolygon: 5,
  Mesh: 6,
  0: "Point",
  1: "LineString",
  2: "Polygon",
  3: "MultiPoint",
  4: "MultiLineString",
  5: "MultiPolygon",
  6: "Mesh",
});

export function decodeTileColumns(data, options = {}) {
  return { layers: decodeRawTileLayers(data, { layers: options.layers }) };
}

export function columnValue(column, index) {
  if ("scope" in column && column.scope !== undefined) {
    throw new TypeError(
      "m-value columns are sparse per vertex or triangle; use mValueSlice instead of columnValue",
    );
  }
  const { values, present } = column;
  if (!Number.isInteger(index) || index < 0 || index >= values.length) {
    throw new RangeError(`the column has no feature ${index}`);
  }
  const has =
    present === undefined || ((present[index >> 3] >> (index & 7)) & 1) === 1;
  return has ? values[index] : undefined;
}

const mValueOffsets = new WeakMap();

function itemCount(geometry, scope, featureIndex) {
  if (scope === "triangle") {
    const offsets = geometry.triangleOffsets;
    return offsets === undefined
      ? 0
      : offsets[featureIndex + 1] - offsets[featureIndex];
  }
  const at = (level, i) => (level === undefined ? i : level[i]);
  const v0 = at(
    geometry.ringOffsets,
    at(geometry.partOffsets, at(geometry.geometryOffsets, featureIndex)),
  );
  const v1 = at(
    geometry.ringOffsets,
    at(geometry.partOffsets, at(geometry.geometryOffsets, featureIndex + 1)),
  );
  return v1 - v0;
}

export function mValueSlice(layer, column, featureIndex) {
  if (
    !Number.isInteger(featureIndex) ||
    featureIndex < 0 ||
    featureIndex >= layer.featureCount
  ) {
    throw new RangeError(
      `layer "${layer.name}" has no feature ${featureIndex}`,
    );
  }
  const { present } = column;
  const has =
    present === undefined ||
    ((present[featureIndex >> 3] >> (featureIndex & 7)) & 1) === 1;
  if (!has) return undefined;

  let byColumn = mValueOffsets.get(layer);
  if (byColumn === undefined) {
    byColumn = new WeakMap();
    mValueOffsets.set(layer, byColumn);
  }
  let offsets = byColumn.get(column);
  if (offsets === undefined) {
    offsets = new Uint32Array(layer.featureCount + 1);
    let total = 0;
    for (let f = 0; f < layer.featureCount; f++) {
      offsets[f] = total;
      if (
        present === undefined ||
        ((present[f >> 3] >> (f & 7)) & 1) === 1
      ) {
        total += itemCount(layer.geometry, column.scope, f);
      }
    }
    offsets[layer.featureCount] = total;
    byColumn.set(column, offsets);
  }

  const start = offsets[featureIndex];
  const end = offsets[featureIndex + 1];
  if (column.type === "string") {
    const out = new Array(end - start);
    for (let i = start; i < end; i++) {
      out[i - start] = column.dictionary[column.codes[i]];
    }
    return out;
  }
  return column.values.subarray(start, end);
}

export function toElevation(z, zStep, zDatum = "elevation") {
  const base = zDatum === "ground" ? 0 : -10000;
  const power = 10 ** Math.abs(zStep);
  const metres = zStep < 0 ? (z + base * power) / power : base + z * power;
  return metres === 0 ? 0 : metres;
}
