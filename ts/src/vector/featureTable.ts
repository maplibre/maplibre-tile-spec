import type { HeightMode, MltGeometryColumns, MltMValueColumn } from "../columns";
import { geometryStart, isTrianglesOnly, lineStart, meshTriangles } from "../featureGeometry";
import { GEOMETRY_TYPE } from "./geometry/geometryType";
import type { Geometry, GeometryVector } from "./geometry/geometryVector";
import type Vector from "./vector";
import type { IdVector } from "./idVector";
import { Int32FlatVector } from "./flat/int32FlatVector";
import { DoubleFlatVector } from "./flat/doubleFlatVector";
import { Int32SequenceVector } from "./sequence/int32SequenceVector";
import { Int32ConstVector } from "./constant/int32ConstVector";
import { GpuVector } from "./geometry/gpuVector";

export interface Feature {
    id: number | bigint | undefined;
    geometry: Geometry;
    properties: { [key: string]: unknown };
    z?: number[];
    triangles?: [number, number, number][];
}

export interface FeatureTableV2Meta {
    version: 1 | 2;
    zStep?: number;
    heightMode?: HeightMode;
    vertexValues: readonly MltMValueColumn[];
    triangleValues: readonly MltMValueColumn[];
    mValues: readonly MltMValueColumn[];
    columnGeometry: MltGeometryColumns;
}

export default class FeatureTable {
    private propertyVectorsMap?: Map<string, Vector>;

    constructor(
        private readonly _name: string,
        private readonly _geometryVector: GeometryVector | GpuVector,
        private readonly _idVector?: IdVector,
        private readonly _propertyVectors?: Vector[],
        private readonly _extent = 4096,
        private readonly _v2Meta?: FeatureTableV2Meta,
    ) {
        if (_name.length === 0) {
            throw new Error("Missing layer name");
        }
    }

    get name(): string {
        return this._name;
    }

    get idVector(): IdVector | undefined {
        return this._idVector;
    }

    get geometryVector(): GeometryVector | GpuVector {
        return this._geometryVector;
    }

    get propertyVectors(): Vector[] {
        return this._propertyVectors ?? [];
    }

    getPropertyVector(name: string): Vector | undefined {
        if (!this.propertyVectorsMap) {
            this.propertyVectorsMap = new Map(this.propertyVectors.map((vector) => [vector.name, vector]));
        }

        return this.propertyVectorsMap.get(name);
    }

    get numFeatures(): number {
        return this.geometryVector.numGeometries;
    }

    get extent(): number {
        return this._extent;
    }

    get version(): 1 | 2 {
        return this._v2Meta?.version ?? 1;
    }

    get zStep(): number | undefined {
        return this._v2Meta?.zStep;
    }

    get heightMode(): HeightMode | undefined {
        return this._v2Meta?.heightMode;
    }

    get vertexValues(): readonly MltMValueColumn[] {
        return this._v2Meta?.vertexValues ?? [];
    }

    get triangleValues(): readonly MltMValueColumn[] {
        return this._v2Meta?.triangleValues ?? [];
    }

    get mValues(): readonly MltMValueColumn[] {
        return this._v2Meta?.mValues ?? [];
    }

    /**
     * Returns all features as an array
     */
    getFeatures(): Feature[] {
        const features: Feature[] = [];
        const geometries = this.geometryVector.getGeometries();
        const colGeom = this._v2Meta?.columnGeometry;
        const trisOnly =
            (this.geometryVector instanceof GpuVector && this.geometryVector.isTrianglesOnly) ||
            (colGeom !== undefined && isTrianglesOnly(colGeom));

        for (let i = 0; i < this.numFeatures; i++) {
            let id: number | bigint | undefined;
            if (this.idVector) {
                const idValue = this.idVector.getValue(i);
                if (idValue !== null) {
                    id = this.containsMaxSafeIntegerValues(this.idVector) ? Number(idValue) : idValue;
                }
            }
            const geomType = this.geometryVector.geometryType(i);
            const geometry: Geometry = {
                coordinates: geometries[i],
                type: geomType,
            };
            if (trisOnly) {
                geometry.trianglesOnly = true;
            } else if (geomType === GEOMETRY_TYPE.MULTIPOLYGON && colGeom?.partOffsets !== undefined) {
                const g0 = geometryStart(colGeom, i);
                const g1 = geometryStart(colGeom, i + 1);
                const counts = new Array<number>(g1 - g0);
                for (let g = g0; g < g1; g++) {
                    counts[g - g0] = colGeom.partOffsets[g + 1] - colGeom.partOffsets[g];
                }
                geometry.ringsPerPolygon = counts;
            }

            const properties: { [key: string]: unknown } = {};
            for (const propertyColumn of this.propertyVectors) {
                if (!propertyColumn) continue;
                const columnName = propertyColumn.name;
                const propertyValue = propertyColumn.getValue(i);
                if (propertyValue !== null) {
                    properties[columnName] = propertyValue;
                }
            }

            const feature: Feature = { id, geometry, properties };

            if (colGeom !== undefined && colGeom.dimension === 3) {
                if (trisOnly && colGeom.triangleOffsets && colGeom.indexBuffer) {
                    const lo = colGeom.triangleOffsets[i] * 3;
                    const hi = colGeom.triangleOffsets[i + 1] * 3;
                    const z = new Array<number>(hi - lo);
                    for (let k = lo; k < hi; k++) {
                        z[k - lo] = colGeom.vertices[colGeom.indexBuffer[k] * 3 + 2];
                    }
                    feature.z = z;
                } else {
                    const g0 = geometryStart(colGeom, i);
                    const g1 = geometryStart(colGeom, i + 1);
                    const v0 = lineStart(colGeom, g0);
                    const v1 = lineStart(colGeom, g1);
                    const z = new Array<number>(v1 - v0);
                    for (let v = v0; v < v1; v++) {
                        z[v - v0] = colGeom.vertices[v * 3 + 2];
                    }
                    feature.z = z;
                }
            }

            if (colGeom !== undefined && geomType === GEOMETRY_TYPE.MESH) {
                const flat = meshTriangles(colGeom, i);
                const tris = new Array<[number, number, number]>(flat.length / 3);
                for (let t = 0; t < tris.length; t++) {
                    tris[t] = [flat[t * 3], flat[t * 3 + 1], flat[t * 3 + 2]];
                }
                feature.triangles = tris;
            }

            features.push(feature);
        }
        return features;
    }

    private containsMaxSafeIntegerValues(idVector: IdVector) {
        return (
            idVector instanceof Int32FlatVector ||
            idVector instanceof Int32ConstVector ||
            idVector instanceof Int32SequenceVector ||
            idVector instanceof DoubleFlatVector
        );
    }
}
