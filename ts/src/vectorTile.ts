import Point from "@mapbox/point-geometry";
import {
    columnValue,
    type DecodeTileColumnsOptions,
    decodeTileColumns,
    type HeightMode,
    type MltColumnLayer,
    type MltGeometryLayout,
    type MltGeometryType,
    type MltMValueColumn,
    type MltMValueSlice,
    type MltPropertyColumn,
    mValueSlice,
} from "./columns";
import { geometryStart, lineStart, meshTriangles, polygonLevels, runLength } from "./featureGeometry";

export { Point as MltPoint };

const POINT = 1;
const LINESTRING = 2;
const POLYGON = 3;
const MVT_TYPES = [POINT, LINESTRING, POLYGON, POINT, LINESTRING, POLYGON, 0] as const;

export type Position3D = [x: number, y: number, z: number];
export type DecodeTileOptions = DecodeTileColumnsOptions;
type VertexOf<T> = (i: number) => T;

function readRun<T>(vertex: VertexOf<T>, start: number, end: number, close: boolean): T[] {
    const count = runLength(start, end);
    if (count === 0) return [];
    const run = new Array<T>(close ? count + 1 : count);
    for (let k = 0; k < count; k++) run[k] = vertex(start + k);
    if (close) run[count] = vertex(start);
    return run;
}

function closedRings<T>(rings: Uint32Array, r0: number, r1: number, vertex: VertexOf<T>): T[][] {
    const out = new Array<T[]>(r1 - r0);
    for (let r = r0; r < r1; r++) out[r - r0] = readRun(vertex, rings[r], rings[r + 1], true);
    return out;
}

interface LayerData {
    readonly layer: MltColumnLayer;
    readonly layout: MltGeometryLayout;
    readonly vertexValues: ReadonlyMap<string, MltMValueColumn>;
    readonly triangleValues: ReadonlyMap<string, MltMValueColumn>;
}

const NO_COLUMNS: ReadonlyMap<string, MltMValueColumn> = new Map();

function byName(columns: readonly MltMValueColumn[]): ReadonlyMap<string, MltMValueColumn> {
    if (columns.length === 0) return NO_COLUMNS;
    const map = new Map<string, MltMValueColumn>();
    for (const col of columns) if (!map.has(col.name)) map.set(col.name, col);
    return map;
}

abstract class FeatureBase<V> {
    readonly extent: number;

    constructor(
        protected readonly _featureIdx: number,
        protected readonly _layer: LayerData,
    ) {
        this.extent = _layer.layer.extent;
    }

    get mltType(): MltGeometryType {
        return this._layer.layer.geometry.types[this._featureIdx];
    }

    get type(): 0 | 1 | 2 | 3 {
        return MVT_TYPES[this.mltType] ?? 0;
    }

    get id(): number | bigint | undefined {
        const { ids } = this._layer.layer;
        const val = ids ? columnValue(ids, this._featureIdx) : null;
        return val === null ? undefined : (val as number | bigint);
    }

    get properties(): Record<string, number | bigint | string | boolean> {
        const result: Record<string, number | bigint | string | boolean> = {};
        for (const col of this._layer.layer.properties) {
            const val = columnValue(col, this._featureIdx);
            if (val !== null) result[col.name] = val;
        }
        return result;
    }

    loadTriangles(): Uint32Array {
        if (this._layer.layout !== "meshes") {
            throw new Error(`feature ${this._featureIdx} of layer "${this._layer.layer.name}" is not a mesh`);
        }
        return meshTriangles(this._layer.layer.geometry, this._featureIdx);
    }

    loadVertexValues(name: string): MltMValueSlice | undefined {
        const col = this._layer.vertexValues.get(name);
        return col ? (mValueSlice(col, this._featureIdx) ?? undefined) : undefined;
    }

    loadTriangleValues(name: string): MltMValueSlice | undefined {
        const col = this._layer.triangleValues.get(name);
        return col ? (mValueSlice(col, this._featureIdx) ?? undefined) : undefined;
    }

    loadMValues(name: string): MltMValueSlice | undefined {
        const col = this._layer.vertexValues.get(name) ?? this._layer.triangleValues.get(name);
        return col ? (mValueSlice(col, this._featureIdx) ?? undefined) : undefined;
    }

    protected abstract vertex(): VertexOf<V>;

    protected geometries(): [start: number, end: number] {
        if (this._layer.layout === "tessPolygons") {
            throw new Error(
                `layer "${this._layer.layer.name}" is a TessPolygons layer, which stores triangles, not outlines`,
            );
        }
        const { geometry } = this._layer.layer;
        return [geometryStart(geometry, this._featureIdx), geometryStart(geometry, this._featureIdx + 1)];
    }

    protected rings(): V[][] {
        const { geometry } = this._layer.layer;
        const vertex = this.vertex();
        const [g0, g1] = this.geometries();
        if (this._layer.layout === "meshes") return [readRun(vertex, g0, g1, false)];
        if (this.type === POINT) {
            const v0 = lineStart(geometry, g0);
            const v1 = lineStart(geometry, g1);
            const pts = new Array<V[]>(runLength(v0, v1));
            for (let v = v0; v < v1; v++) pts[v - v0] = [vertex(v)];
            return pts;
        }
        if (this.type === LINESTRING) {
            const lines = new Array<V[]>(g1 - g0);
            for (let line = g0; line < g1; line++) {
                lines[line - g0] = readRun(vertex, lineStart(geometry, line), lineStart(geometry, line + 1), false);
            }
            return lines;
        }
        if (this.type === POLYGON) {
            const [parts, rings] = polygonLevels(geometry);
            return closedRings(rings, parts[g0], parts[g1], vertex);
        }
        return [];
    }

    protected polygons(): V[][][] {
        if (this._layer.layout === "meshes") {
            throw new Error(
                `feature ${this._featureIdx} of layer "${this._layer.layer.name}" is a mesh; use loadGeometry() and loadTriangles() instead of loadPolygons()`,
            );
        }
        if (this.type !== POLYGON) return [this.rings()];
        const { geometry } = this._layer.layer;
        const vertex = this.vertex();
        const [g0, g1] = this.geometries();
        const [parts, rings] = polygonLevels(geometry);
        const out = new Array<V[][]>(g1 - g0);
        for (let p = g0; p < g1; p++) out[p - g0] = closedRings(rings, parts[p], parts[p + 1], vertex);
        return out;
    }
}

export class MltFeature extends FeatureBase<Point> {
    readonly zStep: number | undefined = this._layer.layer.geometry.zStep;
    readonly heightMode: HeightMode | undefined = this._layer.layer.geometry.heightMode;

    loadGeometry(): Point[][] {
        return this.rings();
    }

    loadZ(): number[] {
        const { geometry } = this._layer.layer;
        if (geometry.dimension !== 3) return [];
        const [g0, g1] = this.geometries();
        return readRun((v) => geometry.vertices[v * 3 + 2], lineStart(geometry, g0), lineStart(geometry, g1), false);
    }

    loadPolygons(): Point[][][] {
        return this.polygons();
    }

    protected vertex(): VertexOf<Point> {
        const { vertices, dimension } = this._layer.layer.geometry;
        return (i) => new Point(vertices[i * dimension], vertices[i * dimension + 1]);
    }
}

export class MltFeature3D extends FeatureBase<Position3D> {
    constructor(
        featureIdx: number,
        layer: LayerData,
        readonly zStep: number,
        readonly heightMode: HeightMode = "absolute",
    ) {
        super(featureIdx, layer);
    }

    loadGeometry(): Position3D[][] {
        return this.rings();
    }

    loadPolygons(): Position3D[][][] {
        return this.polygons();
    }

    protected vertex(): VertexOf<Position3D> {
        const { vertices } = this._layer.layer.geometry;
        return (i) => [vertices[i * 3], vertices[i * 3 + 1], vertices[i * 3 + 2]];
    }
}

abstract class LayerBase {
    readonly name: string;
    readonly extent: number;
    readonly length: number;
    readonly propertyColumns: readonly MltPropertyColumn[];
    readonly propertyKeys: readonly string[];
    readonly vertexValues: readonly MltMValueColumn[];
    readonly triangleValues: readonly MltMValueColumn[];
    readonly mValues: readonly MltMValueColumn[];
    protected readonly _data: LayerData;

    constructor(layer: MltColumnLayer) {
        this._data = {
            layer,
            layout: layer.geometry.layout,
            vertexValues: byName(layer.vertexValues),
            triangleValues: byName(layer.triangleValues),
        };
        this.name = layer.name;
        this.extent = layer.extent;
        this.length = layer.featureCount;
        this.propertyColumns = layer.properties;
        this.propertyKeys = layer.properties.map((c) => c.name);
        this.vertexValues = layer.vertexValues;
        this.triangleValues = layer.triangleValues;
        this.mValues = layer.mValues;
    }

    protected featureIndex(i: number): number {
        if (!Number.isInteger(i) || i < 0 || i >= this.length) {
            throw new RangeError(`layer "${this.name}" has no feature ${i}`);
        }
        return i;
    }
}

export class MltLayer extends LayerBase {
    readonly version = 1 as const;
    readonly zStep: number | undefined = this._data.layer.geometry.zStep;
    readonly heightMode: HeightMode | undefined = this._data.layer.geometry.heightMode;

    feature(i: number): MltFeature {
        return new MltFeature(this.featureIndex(i), this._data);
    }
}

export class MltLayer3D extends LayerBase {
    readonly zStep: number;
    readonly heightMode: HeightMode;

    constructor(layer: MltColumnLayer) {
        super(layer);
        const { zStep, heightMode } = layer.geometry;
        if (zStep === undefined || heightMode === undefined) {
            throw new Error(
                `layer "${layer.name}" has no z coordinates; decode the tile with decodeVectorTile() instead`,
            );
        }
        const { layout } = this._data;
        if (layout === "tessPolygons" || layout === "tessPolygonsWithOutlines") {
            throw new Error(
                `layer "${layer.name}" uses the ${layout === "tessPolygons" ? "TessPolygons" : "TessPolygonsWithOutlines"} geometry layout, which decodeTile3D does not support`,
            );
        }
        this.zStep = zStep;
        this.heightMode = heightMode;
    }

    feature(i: number): MltFeature3D {
        return new MltFeature3D(this.featureIndex(i), this._data, this.zStep, this.heightMode);
    }
}

function decodeLayers<L>(
    data: Uint8Array,
    Layer: new (layer: MltColumnLayer) => L,
    options?: DecodeTileColumnsOptions,
): Record<string, L> {
    const layers: Record<string, L> = {};
    const seen = new Set<string>();
    for (const layer of decodeTileColumns(data, options).layers) {
        if (seen.has(layer.name)) {
            throw new Error(`the tile has two layers named "${layer.name}"; decodeTileColumns keeps both`);
        }
        seen.add(layer.name);
        layers[layer.name] = new Layer(layer);
    }
    return layers;
}

export interface MltTile {
    readonly layers: Record<string, MltLayer>;
}

export interface MltTile3D {
    readonly layers: Record<string, MltLayer3D>;
}

export function decodeVectorTile(data: Uint8Array, options?: DecodeTileColumnsOptions): MltTile {
    return { layers: decodeLayers(data, MltLayer, options) };
}

export function decodeTile3D(data: Uint8Array, options?: DecodeTileColumnsOptions): MltTile3D {
    return { layers: decodeLayers(data, MltLayer3D, options) };
}
