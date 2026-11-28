import { type MltColumnLayer, type MltGeometryColumns, MltGeometryType } from "./columns";

export type MltFeatureGeometry = MltPointGeometry | MltLineGeometry | MltPolygonGeometry | MltMeshGeometry;

interface MltFeatureGeometryBase {
    readonly type: MltGeometryType;
    readonly vertices: Int32Array;
    readonly firstVertex: number;
}

export interface MltPointGeometry extends MltFeatureGeometryBase {
    readonly kind: "point";
}

export interface MltLineGeometry extends MltFeatureGeometryBase {
    readonly kind: "line";
    readonly lines: readonly MltVertexRun[];
}

export interface MltPolygonGeometry extends MltFeatureGeometryBase {
    readonly kind: "polygon";
    readonly polygons: readonly MltPolygonRings[];
    readonly triangles: Uint32Array | undefined;
}

export interface MltMeshGeometry extends MltFeatureGeometryBase {
    readonly kind: "mesh";
    readonly triangles: Uint32Array;
}

export interface MltVertexRun {
    readonly vertices: Int32Array;
    readonly firstVertex: number;
}

export interface MltPolygonRings extends MltVertexRun {
    readonly holeIndices: number[];
}

export interface MltGeometryStarts {
    readonly featureGeometries: Uint32Array;
    readonly geometryVertices: Uint32Array;
    readonly geometryRings?: Uint32Array;
    readonly ringVertices?: Uint32Array;
}

const polygonOrdinals = new WeakMap<MltGeometryColumns, Uint32Array>();
const resolvedStarts = new WeakMap<MltGeometryColumns, MltGeometryStarts>();

function polygonOrdinal(geometry: MltGeometryColumns, index: number): number {
    if (isMeshes(geometry)) return index;
    let ordinals = polygonOrdinals.get(geometry);
    if (!ordinals) {
        const { types } = geometry;
        ordinals = new Uint32Array(types.length);
        let polygons = 0;
        for (let f = 0; f < types.length; f++) {
            ordinals[f] = polygons;
            if (types[f] === MltGeometryType.Polygon || types[f] === MltGeometryType.MultiPolygon) polygons++;
        }
        polygonOrdinals.set(geometry, ordinals);
    }
    return ordinals[index];
}

export function isTrianglesOnly(geometry: MltGeometryColumns): boolean {
    return (
        geometry.layout === "tessPolygons" ||
        (geometry.indexBuffer !== undefined && geometry.partOffsets === undefined && !isMeshes(geometry))
    );
}

export function isMeshes(geometry: MltGeometryColumns): boolean {
    return geometry.layout === "meshes" || geometry.meshes === true || geometry.types[0] === MltGeometryType.Mesh;
}

export function geometryStart(geometry: MltGeometryColumns, index: number): number {
    return geometry.geometryOffsets === undefined ? index : geometry.geometryOffsets[index];
}

export function lineStart(geometry: MltGeometryColumns, line: number): number {
    const p = geometry.partOffsets === undefined ? line : geometry.partOffsets[line];
    return geometry.ringOffsets === undefined ? p : geometry.ringOffsets[p];
}

export function polygonLevels(geometry: MltGeometryColumns): [parts: Uint32Array, rings: Uint32Array] {
    const { partOffsets, ringOffsets } = geometry;
    if (partOffsets === undefined || ringOffsets === undefined) {
        throw new Error("a layer storing polygons has part and ring offsets, and this one has not");
    }
    return [partOffsets, ringOffsets];
}

export function runLength(start: number, end: number): number {
    if (end < start) throw new Error(`a run of vertices ${start}..${end} ends before it starts`);
    return end - start;
}

function view(geometry: MltGeometryColumns, start: number, end: number): Int32Array {
    runLength(start, end);
    return geometry.vertices.subarray(start * geometry.dimension, end * geometry.dimension);
}

function featureCorners(geometry: MltGeometryColumns, index: number): Uint32Array {
    const { indexBuffer, triangleOffsets } = geometry;
    const ord = polygonOrdinal(geometry, index);
    const t0 = triangleOffsets?.[ord];
    const t1 = triangleOffsets?.[ord + 1];
    if (indexBuffer === undefined || t0 === undefined || t1 === undefined) {
        throw new Error(`the layer's triangles do not cover feature ${index}`);
    }
    return indexBuffer.subarray(t0 * 3, t1 * 3);
}

function rebase(corners: Uint32Array, v0: number, v1: number, index: number): Uint32Array {
    const triangles = new Uint32Array(corners.length);
    for (let i = 0; i < corners.length; i++) {
        if (corners[i] < v0 || corners[i] >= v1) {
            throw new Error(`a triangle of feature ${index} names vertex ${corners[i]}, outside ${v0}..${v1}`);
        }
        triangles[i] = corners[i] - v0;
    }
    return triangles;
}

export function meshTriangles(geometry: MltGeometryColumns, index: number): Uint32Array {
    if (!isMeshes(geometry)) throw new Error(`feature ${index} is not a mesh`);
    return rebase(
        featureCorners(geometry, index),
        geometryStart(geometry, index),
        geometryStart(geometry, index + 1),
        index,
    );
}

function identity(count: number): Uint32Array {
    const out = new Uint32Array(count + 1);
    for (let i = 0; i <= count; i++) out[i] = i;
    return out;
}

function checked(name: string, starts: Uint32Array, count: number, end?: number): Uint32Array {
    if (starts.length !== count + 1) throw new Error(`${name} has ${starts.length - 1} items, expected ${count}`);
    if (starts[0] !== 0) throw new Error(`${name} starts at ${starts[0]}, expected 0`);
    for (let i = 0; i < count; i++) runLength(starts[i], starts[i + 1]);
    if (end !== undefined && starts[count] !== end)
        throw new Error(`${name} ends at ${starts[count]}, expected ${end}`);
    return starts;
}

export function geometryStarts(geometry: MltGeometryColumns): MltGeometryStarts {
    let starts = resolvedStarts.get(geometry);
    if (starts !== undefined) return starts;
    if (isTrianglesOnly(geometry)) throw new Error("a TessPolygons layer stores triangles, not runs of vertices");
    const { geometryOffsets, partOffsets, ringOffsets, types, vertices, dimension } = geometry;
    if (vertices.length % dimension !== 0) {
        throw new Error(`the layer has ${vertices.length} vertex words, not a multiple of its dimension ${dimension}`);
    }
    const vertexCount = vertices.length / dimension;
    const featureGeometries = checked("featureGeometries", geometryOffsets ?? identity(types.length), types.length);
    const geometries = featureGeometries[types.length];
    const parts = partOffsets ?? identity(geometries);
    if (ringOffsets === undefined) {
        starts = { featureGeometries, geometryVertices: checked("geometryVertices", parts, geometries, vertexCount) };
    } else {
        const geometryRings = checked("geometryRings", parts, geometries);
        const ringVertices = checked("ringVertices", ringOffsets, geometryRings[geometries], vertexCount);
        const geometryVertices = new Uint32Array(geometries + 1);
        for (let g = 0; g <= geometries; g++) geometryVertices[g] = lineStart(geometry, g);
        starts = { featureGeometries, geometryVertices, geometryRings, ringVertices };
    }
    resolvedStarts.set(geometry, starts);
    return starts;
}

export function featureGeometry(layer: MltColumnLayer, index: number): MltFeatureGeometry {
    const { geometry } = layer;
    if (!Number.isInteger(index) || index < 0 || index >= layer.featureCount) {
        throw new RangeError(`layer "${layer.name}" has no feature ${index}`);
    }
    if (isTrianglesOnly(geometry)) {
        throw new Error(`layer "${layer.name}" is a TessPolygons layer, which stores triangles, not outlines`);
    }
    const type = geometry.types[index] as MltGeometryType;
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
            const lines: MltVertexRun[] = [];
            for (let line = g0; line < g1; line++) {
                const a = lineStart(geometry, line);
                lines.push({ vertices: view(geometry, a, lineStart(geometry, line + 1)), firstVertex: a });
            }
            return { kind: "line", type, vertices, firstVertex: v0, lines };
        }
        case MltGeometryType.Polygon:
        case MltGeometryType.MultiPolygon: {
            const [parts, rings] = polygonLevels(geometry);
            const polygons: MltPolygonRings[] = [];
            for (let poly = g0; poly < g1; poly++) {
                const r0 = parts[poly];
                const r1 = parts[poly + 1];
                const a = rings[r0];
                const b = rings[r1];
                const holeIndices: number[] = [];
                let prev = a;
                for (let ring = r0 + 1; ring < r1; ring++) {
                    runLength(prev, rings[ring]);
                    prev = rings[ring];
                    holeIndices.push(prev - a);
                }
                runLength(prev, b);
                polygons.push({ vertices: view(geometry, a, b), firstVertex: a, holeIndices });
            }
            const triangles =
                geometry.indexBuffer === undefined ? undefined : rebase(featureCorners(geometry, index), v0, v1, index);
            return { kind: "polygon", type, vertices, firstVertex: v0, polygons, triangles };
        }
        case MltGeometryType.Mesh:
            return { kind: "mesh", type, vertices, firstVertex: v0, triangles: meshTriangles(geometry, index) };
        default:
            throw new Error(`feature ${index} of layer "${layer.name}" has unknown geometry type ${type}`);
    }
}
