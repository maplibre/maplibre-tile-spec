import { mValueSlice } from "../columns";
import { mValueSliceToJson } from "../geojson";
import { GEOMETRY_TYPE } from "./geometry/geometryType";
import { classifyRings } from "./geometry/classifyRings";
import type { Geometry } from "./geometry/geometryVector";
import type FeatureTable from "./featureTable";

/**
 * Converts decoded feature tables into the GeoJSON shape the synthetic fixtures are written in.
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
                if (table.heightMode === "relativeToGround") properties._height_mode = "relative_to_ground";
            }
            for (const mCol of table.mValues) {
                const slice = mValueSlice(mCol, i);
                if (slice !== null) properties[`m:${mCol.name}`] = mValueSliceToJson(mCol, slice, false);
            }

            const geojsonFeature: GeoJSON.Feature & { triangles?: [number, number, number][] } = {
                type: "Feature",
                geometry: getGeometry(feature.geometry, feature.z),
                properties,
            };
            if (feature.id !== undefined) geojsonFeature.id = Number(feature.id);
            if (feature.triangles !== undefined) geojsonFeature.triangles = feature.triangles;
            features.push(geojsonFeature);
        }
    }
    return { type: "FeatureCollection", features };
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
                for (let k = 0; k < stored; k++) out[k] = [ring[k].x, ring[k].y, z[zi++]];
                out[stored] = [...out[0]];
                return out;
            }
            return ring.map((p) => [p.x, p.y, z[zi++]]);
        });
    } else {
        coords = geometry.coordinates.map((ring) => ring.map((p) => [p.x, p.y]));
    }

    if (geometry.trianglesOnly) return { type: "MultiPolygon", coordinates: coords.map((ring) => [ring]) };

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

function safeNumber<T>(val: bigint | T): unknown {
    if (typeof val === "bigint") return Number(val);
    if (Array.isArray(val)) return val.map(safeNumber);
    if (val !== null && typeof val === "object") {
        return Object.fromEntries(Object.entries(val).map(([k, v]) => [k, safeNumber(v)]));
    }
    return val;
}
