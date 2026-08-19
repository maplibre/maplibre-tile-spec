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
        for (const feature of table.getFeatures()) {
            const geojsonFeature: GeoJSON.Feature = {
                type: "Feature",
                geometry: getGeometry(feature.geometry),
                properties: {
                    _layer: table.name,
                    _extent: table.extent,
                    ...Object.fromEntries(Object.entries(feature.properties).map(([k, v]) => [k, safeNumber(v)])),
                },
            };
            // Ids are always numeric, so they convert directly rather than through `safeNumber`,
            // whose nested-value return type GeoJSON's id does not accept.
            if (feature.id !== undefined) {
                geojsonFeature.id = Number(feature.id);
            }
            features.push(geojsonFeature);
        }
    }
    return { type: "FeatureCollection", features };
}

/**
 * Converts one decoded geometry to GeoJSON. Multi-polygons need their flat ring list grouped back
 * into polygons, which {@link classifyRings} does from the winding order.
 */
export function getGeometry(geometry: Geometry): GeoJSON.Geometry {
    const coords = geometry.coordinates.map((ring) => ring.map((p) => [p.x, p.y]));
    switch (geometry.type) {
        case GEOMETRY_TYPE.POINT:
            return { type: "Point", coordinates: coords[0][0] };
        case GEOMETRY_TYPE.LINESTRING:
            return { type: "LineString", coordinates: coords[0] };
        case GEOMETRY_TYPE.POLYGON:
            return { type: "Polygon", coordinates: coords };
        case GEOMETRY_TYPE.MULTIPOINT:
            return { type: "MultiPoint", coordinates: coords.map((r) => r[0]) };
        case GEOMETRY_TYPE.MULTILINESTRING:
            return { type: "MultiLineString", coordinates: coords };
        case GEOMETRY_TYPE.MULTIPOLYGON:
            return { type: "MultiPolygon", coordinates: classifyRings(coords) };
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
