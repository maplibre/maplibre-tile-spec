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
    if (col.kind === "f32" || col.kind === "f64") return floatToJsonValue(val as number, col.kind === "f32");
    return typeof val === "bigint" ? Number(val) : val;
}

export function mValueSliceToJson(col: MltMValueColumn, slice: MltMValueSlice, encodeNonFinite = true): unknown[] {
    if (col.kind === "strDict") {
        const s = slice as { codes: Uint32Array; dictionary: readonly string[] };
        return Array.from(s.codes, (c) => s.dictionary[c] ?? "");
    }
    if (col.kind === "str") return slice as string[];
    if (col.kind === "bool") return Array.from(slice as Uint8Array, (v) => v === 1);
    if (encodeNonFinite && (col.kind === "f32" || col.kind === "f64")) {
        return Array.from(slice as Float32Array | Float64Array, (v) => floatToJsonValue(v, col.kind === "f32"));
    }
    if (col.kind === "i64" || col.kind === "u64") {
        return Array.from(slice as BigInt64Array | BigUint64Array, Number);
    }
    return Array.from(slice as ArrayLike<number>);
}

function buildFeatureGeoJsonGeometry(
    layer: MltColumnLayer,
    f: number,
): { geometry: GeoJSON.Geometry; triangles?: [number, number, number][] } {
    const { geometry } = layer;
    const { vertices, dimension, types } = geometry;
    const pos = (v: number): GeoJSON.Position =>
        dimension === 3
            ? [vertices[v * 3], vertices[v * 3 + 1], vertices[v * 3 + 2]]
            : [vertices[v * 2], vertices[v * 2 + 1]];

    const openLine = (v0: number, v1: number): GeoJSON.Position[] => {
        const out = new Array<GeoJSON.Position>(v1 - v0);
        for (let v = v0; v < v1; v++) out[v - v0] = pos(v);
        return out;
    };

    const closedRing = (r0: number, r1: number): GeoJSON.Position[] => {
        if (r1 <= r0) return [];
        const ring = openLine(r0, r1);
        ring.push(pos(r0));
        return ring;
    };

    const readPoly = (rings: Uint32Array, r0: number, r1: number): GeoJSON.Position[][] => {
        if (r1 <= r0) return [[]];
        const poly = new Array<GeoJSON.Position[]>(r1 - r0);
        for (let r = r0; r < r1; r++) poly[r - r0] = closedRing(rings[r], rings[r + 1]);
        return poly;
    };

    if (isTrianglesOnly(geometry)) {
        const tris = geometry.triangleOffsets;
        const indices = geometry.indexBuffer;
        if (!tris || !indices) throw new Error("TessPolygons layer missing triangleOffsets or indexBuffer");
        const polys: GeoJSON.Position[][][] = [];
        for (let k = tris[f] * 3; k < tris[f + 1] * 3; k += 3) {
            const a = pos(indices[k]);
            polys.push([[a, pos(indices[k + 1]), pos(indices[k + 2]), [...a]]]);
        }
        return { geometry: { type: "MultiPolygon", coordinates: polys } };
    }

    const g0 = geometryStart(geometry, f);
    const g1 = geometryStart(geometry, f + 1);
    const type = types[f] as MltGeometryType;

    switch (type) {
        case MltGeometryType.Point:
            return { geometry: { type: "Point", coordinates: pos(lineStart(geometry, g0)) } };
        case MltGeometryType.MultiPoint: {
            const coords = new Array<GeoJSON.Position>(g1 - g0);
            for (let g = g0; g < g1; g++) coords[g - g0] = pos(lineStart(geometry, g));
            return { geometry: { type: "MultiPoint", coordinates: coords } };
        }
        case MltGeometryType.LineString:
            return {
                geometry: {
                    type: "LineString",
                    coordinates: openLine(lineStart(geometry, g0), lineStart(geometry, g0 + 1)),
                },
            };
        case MltGeometryType.MultiLineString: {
            const lines = new Array<GeoJSON.Position[]>(g1 - g0);
            for (let g = g0; g < g1; g++) lines[g - g0] = openLine(lineStart(geometry, g), lineStart(geometry, g + 1));
            return { geometry: { type: "MultiLineString", coordinates: lines } };
        }
        case MltGeometryType.Polygon: {
            const [parts, rings] = polygonLevels(geometry);
            return { geometry: { type: "Polygon", coordinates: readPoly(rings, parts[g0], parts[g0 + 1]) } };
        }
        case MltGeometryType.MultiPolygon: {
            const [parts, rings] = polygonLevels(geometry);
            const polys = new Array<GeoJSON.Position[][]>(g1 - g0);
            for (let g = g0; g < g1; g++) polys[g - g0] = readPoly(rings, parts[g], parts[g + 1]);
            return { geometry: { type: "MultiPolygon", coordinates: polys } };
        }
        case MltGeometryType.Mesh: {
            const triOff = geometry.triangleOffsets;
            const indices = geometry.indexBuffer;
            if (!triOff || !indices) throw new Error("Mesh layer missing triangleOffsets or indexBuffer");
            const t0 = triOff[f];
            const triangles = new Array<[number, number, number]>(triOff[f + 1] - t0);
            for (let t = t0; t < triOff[f + 1]; t++) {
                triangles[t - t0] = [indices[t * 3] - g0, indices[t * 3 + 1] - g0, indices[t * 3 + 2] - g0];
            }
            return { geometry: { type: "MultiPoint", coordinates: openLine(g0, g1) }, triangles };
        }
        default:
            throw new Error(`unknown geometry type ${type}`);
    }
}

export type MltGeoJsonFeature = GeoJSON.Feature & { triangles?: [number, number, number][] };

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
            const rawProps: Record<string, unknown> = { _layer: layer.name, _extent: layer.extent };
            for (const col of layer.properties) {
                const val = propJsonValue(col, f);
                if (val !== undefined) rawProps[col.name] = val;
            }
            for (const mCol of layer.mValues) {
                const slice = mValueSlice(mCol, f);
                if (slice !== null) rawProps[`m:${mCol.name}`] = mValueSliceToJson(mCol, slice, true);
            }
            for (const nCol of nestedCols) rawProps[nCol.name] = nCol.values[f];
            if (layer.geometry.zStep !== undefined) {
                rawProps._z_step = layer.geometry.zStep;
                if (layer.geometry.heightMode === "relativeToGround") rawProps._height_mode = "relative_to_ground";
            }

            const properties: Record<string, unknown> = {};
            for (const k of Object.keys(rawProps).sort()) properties[k] = rawProps[k];

            const rawId = layer.ids ? columnValue(layer.ids, f) : null;
            const id = rawId === null ? undefined : Number(rawId);
            const { geometry, triangles } = buildFeatureGeoJsonGeometry(layer, f);

            const feature: MltGeoJsonFeature = { type: "Feature", properties, geometry };
            if (id !== undefined) feature.id = id;
            if (triangles !== undefined) feature.triangles = triangles;
            features.push(feature);
        }
    }

    return { type: "FeatureCollection", features };
}
