import type GeoJSON from "geojson";
import {
    columnValue,
    type DecodeTileColumnsOptions,
    type MltColumnLayer,
    MltGeometryType,
    type MltMValueColumn,
    type MltMValueSlice,
    type MltPropertyColumn,
    mValueSlice,
} from "./columns";
import { decodeRawTileLayers } from "./decoding/v2/layer";
import { floatToJsonValue } from "./decoding/v2/nested";
import { geometryStart, isTrianglesOnly, lineStart, polygonLevels } from "./featureGeometry";

function propJsonValue(col: MltPropertyColumn, f: number): unknown {
    const val = columnValue(col, f);
    if (val === null) return undefined;
    if (col.kind === "f32") return floatToJsonValue(val as number, true);
    if (col.kind === "f64") return floatToJsonValue(val as number, false);
    if (typeof val === "bigint") return Number(val);
    return val;
}

function mValueToJson(col: MltMValueColumn, slice: MltMValueSlice): unknown[] {
    if (col.kind === "strDict") {
        const s = slice as { codes: Uint32Array; dictionary: readonly string[] };
        return Array.from(s.codes, (c) => s.dictionary[c] ?? "");
    }
    if (col.kind === "str") return slice as string[];
    if (col.kind === "bool") return Array.from(slice as Uint8Array, (v) => v === 1);
    if (col.kind === "f32") {
        return Array.from(slice as Float32Array, (v) => floatToJsonValue(v, true));
    }
    if (col.kind === "f64") {
        return Array.from(slice as Float64Array, (v) => floatToJsonValue(v, false));
    }
    if (col.kind === "i64" || col.kind === "u64") {
        return Array.from(slice as BigInt64Array | BigUint64Array, (v) => Number(v));
    }
    return Array.from(slice as ArrayLike<number>);
}

function buildFeatureGeoJsonGeometry(
    layer: MltColumnLayer,
    f: number,
): { geometry: GeoJSON.Geometry; triangles?: [number, number, number][] } {
    const { geometry } = layer;
    const { vertices, dimension, types } = geometry;
    const hasZ = dimension === 3;

    const pos = (v: number): GeoJSON.Position =>
        hasZ ? [vertices[v * 3], vertices[v * 3 + 1], vertices[v * 3 + 2]] : [vertices[v * 2], vertices[v * 2 + 1]];

    const closedRing = (r0: number, r1: number): GeoJSON.Position[] => {
        if (r1 <= r0) return [];
        const ring = new Array<GeoJSON.Position>(r1 - r0 + 1);
        for (let v = r0; v < r1; v++) ring[v - r0] = pos(v);
        ring[r1 - r0] = pos(r0);
        return ring;
    };

    if (isTrianglesOnly(geometry)) {
        const tris = geometry.triangleOffsets;
        const indices = geometry.indexBuffer;
        if (!tris || !indices) {
            throw new Error("TessPolygons layer missing triangleOffsets or indexBuffer");
        }
        const lo = tris[f] * 3;
        const hi = tris[f + 1] * 3;
        const polys: GeoJSON.Position[][][] = [];
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
    const type = types[f] as MltGeometryType;

    switch (type) {
        case MltGeometryType.Point:
            return {
                geometry: { type: "Point", coordinates: pos(lineStart(geometry, g0)) },
            };
        case MltGeometryType.MultiPoint: {
            const coords = new Array<GeoJSON.Position>(g1 - g0);
            for (let g = g0; g < g1; g++) coords[g - g0] = pos(lineStart(geometry, g));
            return { geometry: { type: "MultiPoint", coordinates: coords } };
        }
        case MltGeometryType.LineString: {
            const v0 = lineStart(geometry, g0);
            const v1 = lineStart(geometry, g0 + 1);
            const coords = new Array<GeoJSON.Position>(v1 - v0);
            for (let v = v0; v < v1; v++) coords[v - v0] = pos(v);
            return { geometry: { type: "LineString", coordinates: coords } };
        }
        case MltGeometryType.MultiLineString: {
            const lines = new Array<GeoJSON.Position[]>(g1 - g0);
            for (let g = g0; g < g1; g++) {
                const v0 = lineStart(geometry, g);
                const v1 = lineStart(geometry, g + 1);
                const line = new Array<GeoJSON.Position>(v1 - v0);
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
            const poly = new Array<GeoJSON.Position[]>(r1 - r0);
            for (let r = r0; r < r1; r++) poly[r - r0] = closedRing(rings[r], rings[r + 1]);
            return { geometry: { type: "Polygon", coordinates: poly } };
        }
        case MltGeometryType.MultiPolygon: {
            const [parts, rings] = polygonLevels(geometry);
            const polys = new Array<GeoJSON.Position[][]>(g1 - g0);
            for (let g = g0; g < g1; g++) {
                const r0 = parts[g];
                const r1 = parts[g + 1];
                if (r1 <= r0) {
                    polys[g - g0] = [[]];
                } else {
                    const poly = new Array<GeoJSON.Position[]>(r1 - r0);
                    for (let r = r0; r < r1; r++) {
                        poly[r - r0] = closedRing(rings[r], rings[r + 1]);
                    }
                    polys[g - g0] = poly;
                }
            }
            return { geometry: { type: "MultiPolygon", coordinates: polys } };
        }
        case MltGeometryType.Mesh: {
            const coords = new Array<GeoJSON.Position>(g1 - g0);
            for (let v = g0; v < g1; v++) coords[v - g0] = pos(v);
            const triOff = geometry.triangleOffsets;
            const indices = geometry.indexBuffer;
            if (!triOff || !indices) {
                throw new Error("Mesh layer missing triangleOffsets or indexBuffer");
            }
            const t0 = triOff[f];
            const t1 = triOff[f + 1];
            const triangles = new Array<[number, number, number]>(t1 - t0);
            for (let t = t0; t < t1; t++) {
                triangles[t - t0] = [indices[t * 3] - g0, indices[t * 3 + 1] - g0, indices[t * 3 + 2] - g0];
            }
            return { geometry: { type: "MultiPoint", coordinates: coords }, triangles };
        }
        default:
            throw new Error(`unknown geometry type ${type}`);
    }
}

export type MltGeoJsonFeature = GeoJSON.Feature & {
    triangles?: [number, number, number][];
};

/**
 * Decode an MLT tile (v1, v2, 3D Z, `Meshes`, `m-values`, and `nested`) into a GeoJSON
 * `FeatureCollection` matching `mlt-core::FeatureCollection::from_layers`.
 */
export function tileGeoJson(data: Uint8Array, options: DecodeTileColumnsOptions = {}): GeoJSON.FeatureCollection {
    const rawLayers = decodeRawTileLayers(data, {
        includeNested: options.includeNested !== false,
        nestedJsonMode: true,
        maxBytes: options.maxBytes,
        layers: options.layers,
    });
    const features: MltGeoJsonFeature[] = [];

    for (const layer of rawLayers) {
        const nestedCols = layer.nested ?? [];
        for (let f = 0; f < layer.featureCount; f++) {
            const rawProps: Record<string, unknown> = {};
            for (const col of layer.properties) {
                const val = propJsonValue(col, f);
                if (val !== undefined) rawProps[col.name] = val;
            }
            for (const mCol of layer.mValues) {
                const slice = mValueSlice(mCol, f);
                if (slice !== null) rawProps[`m:${mCol.name}`] = mValueToJson(mCol, slice);
            }
            for (const nCol of nestedCols) {
                rawProps[nCol.name] = nCol.values[f];
            }
            if (layer.geometry.zStep !== undefined) {
                rawProps._z_step = layer.geometry.zStep;
                if (layer.geometry.heightMode === "relativeToGround") {
                    rawProps._height_mode = "relative_to_ground";
                }
            }
            rawProps._layer = layer.name;
            rawProps._extent = layer.extent;

            const properties: Record<string, unknown> = {};
            for (const k of Object.keys(rawProps).sort()) properties[k] = rawProps[k];

            const rawId = layer.ids ? columnValue(layer.ids, f) : null;
            const id = rawId === null ? undefined : typeof rawId === "bigint" ? Number(rawId) : (rawId as number);
            const { geometry, triangles } = buildFeatureGeoJsonGeometry(layer, f);

            const feature: MltGeoJsonFeature = {
                type: "Feature",
                properties,
                geometry,
            };
            if (id !== undefined) feature.id = id;
            if (triangles !== undefined) feature.triangles = triangles;
            features.push(feature);
        }
    }

    return { type: "FeatureCollection", features };
}
