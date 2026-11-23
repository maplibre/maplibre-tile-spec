import { type MltMValueColumn, type MltMValueSlice, mValueSlice } from "../columns";
import { GEOMETRY_TYPE } from "./geometry/geometryType";
import { classifyRings } from "./geometry/classifyRings";
import type { Geometry } from "./geometry/geometryVector";
import type FeatureTable from "./featureTable";

/**
 * Converts decoded feature tables into the GeoJSON shape the synthetic fixtures are written in.
 *
 * Shared by the decoder's synthetic tests and the encoder's round trip, so both compare against the
 * fixtures in exactly the same way. The layer name and extent are carried in the properties, since
 * GeoJSON has nowhere else to put them.
 */
export function featureTablesToFeatureCollection(featureTables: FeatureTable[]): GeoJSON.FeatureCollection {
    const features: GeoJSON.Feature[] = [];
    for (const table of featureTables) {
        const tableFeatures = table.getFeatures();
        for (let i = 0; i < tableFeatures.length; i++) {
            const feature = tableFeatures[i];
            const properties: Record<string, unknown> = {
                _layer: table.name,
                _extent: table.extent,
                ...Object.fromEntries(Object.entries(feature.properties).map(([k, v]) => [k, safeNumber(v)])),
            };

            if (table.zStep !== undefined) {
                properties._z_step = table.zStep;
                if (table.heightMode === "relativeToGround") {
                    properties._height_mode = "relative_to_ground";
                }
            }

            for (const mCol of table.mValues) {
                const slice = mValueSlice(mCol, i);
                if (slice !== null) {
                    properties[`m:${mCol.name}`] = convertMValueSlice(mCol, slice);
                }
            }

            const geojsonFeature: GeoJSON.Feature & { triangles?: [number, number, number][] } = {
                type: "Feature",
                geometry: getGeometry(feature.geometry, feature.z),
                properties,
            };
            // Ids are always numeric, so they convert directly rather than through `safeNumber`,
            // whose nested-value return type GeoJSON's id does not accept.
            if (feature.id !== undefined) {
                geojsonFeature.id = Number(feature.id);
            }
            if (feature.triangles !== undefined) {
                geojsonFeature.triangles = feature.triangles;
            }
            features.push(geojsonFeature);
        }
    }
    return { type: "FeatureCollection", features };
}

function convertMValueSlice(col: MltMValueColumn, slice: MltMValueSlice): unknown[] {
    if (col.kind === "strDict") {
        const s = slice as { codes: Uint32Array; dictionary: readonly string[] };
        return Array.from(s.codes, (c) => s.dictionary[c] ?? "");
    }
    if (col.kind === "str") return slice as string[];
    if (col.kind === "bool") return Array.from(slice as Uint8Array, (v) => v === 1);
    if (col.kind === "i64" || col.kind === "u64") {
        return Array.from(slice as BigInt64Array | BigUint64Array, (v) => Number(v));
    }
    return Array.from(slice as ArrayLike<number>);
}

function groupRingsByCount(coords: number[][][], counts: number[]): number[][][][] {
    const polys: number[][][][] = new Array(counts.length);
    let r = 0;
    for (let p = 0; p < counts.length; p++) {
        const count = counts[p];
        if (count === 0) {
            polys[p] = coords[r] && coords[r].length === 0 ? [coords[r++]] : [[]];
        } else {
            polys[p] = coords.slice(r, r + count);
            r += count;
        }
    }
    return polys;
}

/**
 * Converts one decoded geometry to GeoJSON. Multi-polygons need their flat ring list grouped back
 * into polygons, using topological `ringsPerPolygon` when available or {@link classifyRings}.
 */
export function getGeometry(geometry: Geometry, z?: number[]): GeoJSON.Geometry {
    let coords: number[][][];
    if (z !== undefined) {
        let zi = 0;
        const isClosed =
            geometry.trianglesOnly === true ||
            geometry.type === GEOMETRY_TYPE.POLYGON ||
            geometry.type === GEOMETRY_TYPE.MULTIPOLYGON;
        coords = geometry.coordinates.map((ring) => {
            if (ring.length === 0) return [];
            if (isClosed && ring.length > 1) {
                const stored = ring.length - 1;
                const out = new Array<number[]>(ring.length);
                for (let k = 0; k < stored; k++) {
                    out[k] = [ring[k].x, ring[k].y, z[zi++]];
                }
                out[stored] = [...out[0]];
                return out;
            }
            return ring.map((p) => [p.x, p.y, z[zi++]]);
        });
    } else {
        coords = geometry.coordinates.map((ring) => ring.map((p) => [p.x, p.y]));
    }

    if (geometry.trianglesOnly) {
        return { type: "MultiPolygon", coordinates: coords.map((ring) => [ring]) };
    }

    switch (geometry.type) {
        case GEOMETRY_TYPE.POINT:
            return { type: "Point", coordinates: coords[0][0] };
        case GEOMETRY_TYPE.LINESTRING:
            return { type: "LineString", coordinates: coords[0] };
        case GEOMETRY_TYPE.POLYGON:
            return { type: "Polygon", coordinates: coords };
        case GEOMETRY_TYPE.MULTIPOINT:
        case GEOMETRY_TYPE.MESH:
            return { type: "MultiPoint", coordinates: coords.map((r) => r[0]) };
        case GEOMETRY_TYPE.MULTILINESTRING:
            return { type: "MultiLineString", coordinates: coords };
        case GEOMETRY_TYPE.MULTIPOLYGON:
            return {
                type: "MultiPolygon",
                coordinates: geometry.ringsPerPolygon
                    ? groupRingsByCount(coords, geometry.ringsPerPolygon)
                    : classifyRings(coords),
            };
        default:
            throw new Error(`Unsupported geometry type: ${geometry.type}`);
    }
}

/**
 * Converts BigInt to Number so the result can be JSON-serialized for comparison.
 *
 * Nested property values are maps and lists of arbitrary depth, and 64-bit integers anywhere inside
 * them decode to BigInt, so the conversion has to recurse. Values too large for a double lose
 * precision here, which `compareWithTolerance` absorbs.
 */
function safeNumber<T>(val: bigint | T): unknown {
    if (typeof val === "bigint") return Number(val);
    if (Array.isArray(val)) return val.map(safeNumber);
    if (val !== null && typeof val === "object") {
        return Object.fromEntries(Object.entries(val).map(([k, v]) => [k, safeNumber(v)]));
    }
    return val;
}
