import { EMPTY_I32, EMPTY_U32 } from "../codecs/binary.js";
import { decodeRansVertices } from "../codecs/rans.js";
import { decodeTrees } from "../codecs/tri.js";
import {
  LG_RANS,
  NO_Z,
  ST_DATA_MORTON,
  ST_DATA_VERTEX,
  ST_LEN_GEOM,
  ST_LEN_PARTS,
  ST_LEN_RINGS,
  ST_LEN_TRIS,
  ST_OFF_INDEX,
  ST_OFF_TREE_CYC,
  ST_OFF_TREE_SYM,
  ST_OFF_VERTEX,
  ST_PRESENT,
  decodeStreamI32,
  decodeStreamU32,
} from "./stream.js";

export const GEOM_POINT = 0;
export const GEOM_LINESTRING = 1;
export const GEOM_POLYGON = 2;
export const GEOM_MULTIPOINT = 3;
export const GEOM_MULTILINESTRING = 4;
export const GEOM_MULTIPOLYGON = 5;
export const GEOM_MESH = 6;

const isPolygon = (t) => t === GEOM_POLYGON || t === GEOM_MULTIPOLYGON;
const isLine = (t) => t === GEOM_LINESTRING || t === GEOM_MULTILINESTRING;

function decodeRootLengths(types, rootLengths, bufferId) {
  const n = types.length;
  const out = new Uint32Array(n + 1);
  let off = 0;
  let k = 0;
  const rLen = rootLengths.length;
  for (let i = 0; i < n; i++) {
    let inc = 1;
    if (types[i] > bufferId) {
      if (k >= rLen) throw new RangeError("geom oob");
      inc = rootLengths[k++];
    }
    off += inc;
    if (off > 0xffffffff) throw new RangeError("overflow");
    out[i + 1] = off;
  }
  return out;
}

function decodeLevel1Lengths(types, rootOffsets, l1Lengths, hasLine) {
  const out = new Uint32Array(rootOffsets[rootOffsets.length - 1] + 1);
  let outPos = 1;
  let off = 0;
  let k = 0;
  const l1Len = l1Lengths.length;

  for (let i = 0; i < types.length; i++) {
    const t = types[i];
    const ng = rootOffsets[i + 1] - rootOffsets[i];
    if (isPolygon(t) || (hasLine && isLine(t))) {
      for (let j = 0; j < ng; j++) {
        if (k >= l1Len) throw new RangeError("geom oob");
        off += l1Lengths[k++];
        if (off > 0xffffffff) throw new RangeError("overflow");
        out[outPos++] = off;
      }
    } else {
      for (let j = 0; j < ng; j++) out[outPos++] = ++off;
    }
  }
  return out;
}

function decodeLevel2Lengths(types, rootOffsets, l1Offsets, l2Lengths) {
  const out = new Uint32Array(l1Offsets[l1Offsets.length - 1] + 1);
  let outPos = 1;
  let off = 0;
  let l1Pos = 0;
  let l2Pos = 0;
  const l2Len = l2Lengths.length;

  for (let i = 0; i < types.length; i++) {
    const t = types[i];
    const ng = rootOffsets[i + 1] - rootOffsets[i];
    if (t !== GEOM_POINT && t !== GEOM_MULTIPOINT) {
      for (let j = 0; j < ng; j++) {
        const np = l1Offsets[l1Pos + 1] - l1Offsets[l1Pos];
        l1Pos++;
        for (let p = 0; p < np; p++) {
          if (l2Pos >= l2Len) throw new RangeError("geom oob");
          off += l2Lengths[l2Pos++];
          if (off > 0xffffffff) throw new RangeError("overflow");
          out[outPos++] = off;
        }
      }
    } else {
      for (let j = 0; j < ng; j++) out[outPos++] = ++off;
      l1Pos += ng;
    }
  }
  return out;
}

export function decodeTopology(types, geoLengths, partLengths, ringLengths) {
  let geometryOffsets = geoLengths;
  let partOffsets = partLengths;
  let ringOffsets = ringLengths;

  if (geometryOffsets !== undefined) {
    geometryOffsets = decodeRootLengths(types, geometryOffsets, GEOM_POLYGON);
    if (partOffsets !== undefined) {
      if (ringOffsets !== undefined) {
        partOffsets = decodeLevel1Lengths(
          types,
          geometryOffsets,
          partOffsets,
          false,
        );
        ringOffsets = decodeLevel2Lengths(
          types,
          geometryOffsets,
          partOffsets,
          ringOffsets,
        );
      } else {
        partOffsets = decodeLevel1Lengths(
          types,
          geometryOffsets,
          partOffsets,
          true,
        );
      }
    }
  } else if (partOffsets !== undefined) {
    if (ringOffsets !== undefined) {
      let hasLine = false;
      for (let i = 0; i < types.length; i++) {
        if (isLine(types[i])) {
          hasLine = true;
          break;
        }
      }
      partOffsets = decodeRootLengths(types, partOffsets, GEOM_LINESTRING);
      ringOffsets = decodeLevel1Lengths(
        types,
        partOffsets,
        ringOffsets,
        hasLine,
      );
    } else {
      partOffsets = decodeRootLengths(types, partOffsets, GEOM_POINT);
    }
  }

  return { geometryOffsets, partOffsets, ringOffsets };
}

export function vertexIndexAt(levels, f) {
  let idx = levels.geometryOffsets ? levels.geometryOffsets[f] : f;
  if (levels.partOffsets) idx = levels.partOffsets[idx];
  if (levels.ringOffsets) idx = levels.ringOffsets[idx];
  return idx;
}

/**
 * Decode geometry streams into the `MltGeometryColumns` shape.
 * Uses the shared scratch arena for all intermediate length/tree/offset streams.
 */
export function decodeGeometry(
  uniformType,
  featureCount,
  typesStream,
  isV2,
  items,
) {
  const isMeshes = uniformType === GEOM_MESH;
  let types;
  if (uniformType >= 0) {
    types = new Uint8Array(featureCount);
    types.fill(uniformType);
  } else {
    const rawTypes = decodeStreamU32(typesStream, true);
    types = new Uint8Array(rawTypes.length);
    for (let i = 0; i < rawTypes.length; i++) {
      const t = rawTypes[i];
      if (t > GEOM_MULTIPOLYGON) throw new Error("geom type");
      types[i] = t;
    }
  }

  let geoLengths;
  let partLengths;
  let ringLengths;
  let triLengths;
  let vertexOffsets;
  let indexBuffer;
  let treeSymbols;
  let treeCycles;
  let vertexStream = null;
  let zStep = NO_Z;
  let zDatum = 0;

  for (let i = 0; i < items.length; i++) {
    const s = items[i];
    switch (s.t) {
      case ST_PRESENT:
        break;
      case ST_DATA_VERTEX:
      case ST_DATA_MORTON:
        if (vertexStream !== null) throw new Error("dup vtx");
        if (s.zStep !== NO_Z) {
          zStep = s.zStep;
          zDatum = s.zDatum;
        }
        vertexStream = s;
        break;
      case ST_OFF_VERTEX:
        if (vertexOffsets !== undefined) throw new Error("dup voff");
        vertexOffsets = decodeStreamU32(s, true);
        break;
      case ST_OFF_INDEX:
        if (indexBuffer !== undefined) throw new Error("dup idx");
        indexBuffer = decodeStreamU32(s, false);
        break;
      case ST_OFF_TREE_SYM:
        if (!isV2 || treeSymbols !== undefined) throw new Error("tsym");
        treeSymbols = decodeStreamU32(s, true);
        break;
      case ST_OFF_TREE_CYC:
        if (!isV2 || treeCycles !== undefined) throw new Error("tcyc");
        treeCycles = decodeStreamU32(s, true);
        break;
      case ST_LEN_GEOM:
        if (geoLengths !== undefined) throw new Error("dup glen");
        geoLengths = decodeStreamU32(s, true);
        break;
      case ST_LEN_PARTS:
        if (partLengths !== undefined) throw new Error("dup plen");
        partLengths = decodeStreamU32(s, true);
        break;
      case ST_LEN_RINGS:
        if (ringLengths !== undefined) throw new Error("dup rlen");
        ringLengths = decodeStreamU32(s, true);
        break;
      case ST_LEN_TRIS:
        if (triLengths !== undefined) throw new Error("dup tlen");
        triLengths = decodeStreamU32(s, true);
        break;
      default:
        throw new Error("geom st");
    }
  }

  if (isMeshes) {
    if (zStep === NO_Z) throw new Error("mesh z");
    if ((geoLengths ? geoLengths.length : 0) !== types.length) {
      throw new Error("mesh glen");
    }
  } else if (zStep !== NO_Z && zDatum === 1) {
    throw new Error("ground z");
  }

  if (treeSymbols !== undefined || treeCycles !== undefined) {
    if (
      treeSymbols === undefined ||
      treeCycles === undefined ||
      indexBuffer !== undefined
    ) {
      throw new Error("tree pair");
    }
    indexBuffer = decodeTrees(treeSymbols, treeCycles);
  }

  if (!isMeshes && indexBuffer !== undefined && partLengths === undefined) {
    if (!isV2) throw new Error("v1 tri");
    for (let i = 0; i < types.length; i++) {
      if (!isPolygon(types[i])) throw new Error("tess non-poly");
    }
  }

  const levels = decodeTopology(types, geoLengths, partLengths, ringLengths);
  const dimension = zStep !== NO_Z ? 3 : 2;
  const hasDict = vertexOffsets !== undefined;

  let vertices;
  if (vertexStream === null) {
    vertices = EMPTY_I32;
  } else if (vertexStream.l === LG_RANS) {
    if (hasDict) throw new Error("rans dict");
    vertices = decodeRansVertices(
      vertexStream.d,
      levels.ringOffsets ?? levels.partOffsets ?? levels.geometryOffsets,
      vertexStream.w,
    );
  } else {
    vertices = decodeStreamI32(vertexStream, hasDict);
  }

  let triangleOffsets;
  if (triLengths !== undefined) {
    triangleOffsets = new Uint32Array(triLengths.length + 1);
    let total = 0;
    for (let i = 0; i < triLengths.length; i++) {
      total += triLengths[i];
      if (total > 0xffffffff) throw new RangeError("overflow");
      triangleOffsets[i + 1] = total;
    }
  }

  if (treeCycles !== undefined) {
    const triCount = indexBuffer.length / 3;
    const totalTris = triangleOffsets
      ? triangleOffsets[triangleOffsets.length - 1]
      : 0;
    if (totalTris !== triCount) throw new Error("tri count");
    const vertCount = vertices.length / dimension;
    for (let i = 0; i < treeCycles.length; i++) {
      if (treeCycles[i] >= vertCount) throw new RangeError("tcyc vtx");
    }
  }

  if (isMeshes) {
    const runs = levels.geometryOffsets ?? new Uint32Array([0]);
    const vertCount = vertices.length / 3;
    const expectedVerts = runs.length > 0 ? runs[runs.length - 1] : 0;
    if (expectedVerts !== vertCount) throw new Error("mesh vtx");
    const triOff = triangleOffsets ?? new Uint32Array([0]);
    if (triOff.length !== runs.length) throw new Error("mesh tlen");
    const indices = indexBuffer ?? EMPTY_U32;
    for (let f = 0; f < types.length; f++) {
      const start = runs[f];
      const end = runs[f + 1];
      const lo = triOff[f] * 3;
      const hi = triOff[f + 1] * 3;
      if (hi > indices.length) throw new Error("mesh idx");
      for (let k = lo; k < hi; k++) {
        const v = indices[k];
        if (v < start || v >= end) throw new RangeError("mesh oob");
      }
    }
  }

  if (hasDict) {
    const dictCount = vertices.length / dimension;
    const nOff = vertexOffsets.length;
    const expanded = new Int32Array(nOff * dimension);
    if (dimension === 2) {
      for (let i = 0; i < nOff; i++) {
        const idx = vertexOffsets[i];
        if (idx >= dictCount) throw new RangeError("vdict");
        const s = idx << 1;
        const d = i << 1;
        expanded[d] = vertices[s];
        expanded[d + 1] = vertices[s + 1];
      }
    } else {
      for (let i = 0; i < nOff; i++) {
        const idx = vertexOffsets[i];
        if (idx >= dictCount) throw new RangeError("vdict");
        const s = idx * 3;
        const d = i * 3;
        expanded[d] = vertices[s];
        expanded[d + 1] = vertices[s + 1];
        expanded[d + 2] = vertices[s + 2];
      }
    }
    vertices = expanded;
  }

  // Rebase v1 feature-local indices to layer vertex indices.
  if (!isV2 && indexBuffer !== undefined) {
    const triOff = triangleOffsets ?? new Uint32Array([0]);
    const starts = [];
    for (let f = 0; f < types.length; f++) {
      if (isPolygon(types[f])) starts.push(vertexIndexAt(levels, f));
    }
    if (triOff.length !== starts.length + 1) throw new Error("v1 triOff");
    for (let p = 0; p < starts.length; p++) {
      const start = starts[p];
      const from = triOff[p] * 3;
      const to = triOff[p + 1] * 3;
      if (to > indexBuffer.length) throw new RangeError("v1 idx");
      for (let k = from; k < to; k++) {
        indexBuffer[k] = (indexBuffer[k] + start) >>> 0;
      }
    }
  }

  const geom = { types, dimension, vertices };
  if (isMeshes) geom.meshes = true;
  if (zStep !== NO_Z) {
    geom.zStep = zStep;
    geom.zDatum = zDatum === 1 ? "ground" : "elevation";
  }
  if (levels.geometryOffsets !== undefined) geom.geometryOffsets = levels.geometryOffsets;
  if (levels.partOffsets !== undefined) geom.partOffsets = levels.partOffsets;
  if (levels.ringOffsets !== undefined) geom.ringOffsets = levels.ringOffsets;
  if (triangleOffsets !== undefined) geom.triangleOffsets = triangleOffsets;
  if (indexBuffer !== undefined) geom.indexBuffer = indexBuffer;
  return geom;
}
