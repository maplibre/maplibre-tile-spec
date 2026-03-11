import type { Geometry, GeometryVector } from "./geometry/geometryVector";
import type Vector from "./vector";
import type { IdVector } from "./idVector";
import { Int32FlatVector } from "./flat/int32FlatVector";
import { DoubleFlatVector } from "./flat/doubleFlatVector";
import { Int32SequenceVector } from "./sequence/int32SequenceVector";
import { Int32ConstVector } from "./constant/int32ConstVector";
import type { GpuVector } from "./geometry/gpuVector";

export interface Feature {
    id: number | bigint;
    geometry: Geometry;
    properties: { [key: string]: unknown };
}

export default class FeatureTable {
    private propertyVectorsMap: Map<string, Vector>;

    constructor(
        private readonly _name: string,
        private readonly _geometryVector: GeometryVector | GpuVector,
        private readonly _idVector?: IdVector,
        private readonly _propertyVectors?: Vector[],
        private readonly _extent = 4096,
    ) {}

    get name(): string {
        return this._name;
    }

    get idVector(): IdVector {
        return this._idVector;
    }

    get geometryVector(): GeometryVector | GpuVector {
        return this._geometryVector;
    }

    get propertyVectors(): Vector[] {
        return this._propertyVectors;
    }

    getPropertyVector(name: string): Vector {
        if (!this.propertyVectorsMap) {
            this.propertyVectorsMap = new Map(this._propertyVectors.map((vector) => [vector.name, vector]));
        }

        return this.propertyVectorsMap.get(name);
    }

    get numFeatures(): number {
        return this.geometryVector.numGeometries;
    }

    get extent(): number {
        return this._extent;
    }

    /**
     * Returns all features as an array
     */
    getFeatures(): Feature[] {
        const features: Feature[] = [];
        const geometries = this.geometryVector.getGeometries();

        for (let i = 0; i < this.numFeatures; i++) {
            let id;
            if (this.idVector) {
                const idValue = this.idVector.getValue(i);
                id = this.containsMaxSafeIntegerValues(this.idVector) && idValue !== null ? Number(idValue) : idValue;
            }
            const geometry = {
                coordinates: geometries[i],
                type: this.geometryVector.geometryType(i),
            };

            const properties: { [key: string]: unknown } = {};
            for (const propertyColumn of this.propertyVectors) {
                if (!propertyColumn) continue;
                const columnName = propertyColumn.name;
                const propertyValue = propertyColumn.getValue(i);
                if (propertyValue !== null) {
                    properties[columnName] = propertyValue;
                }
            }

            features.push({ id, geometry, properties });
        }
        return features;
    }

    private containsMaxSafeIntegerValues(idVector: IdVector) {
        return (
            idVector instanceof Int32FlatVector ||
            (idVector instanceof Int32ConstVector && idVector instanceof Int32SequenceVector) ||
            idVector instanceof DoubleFlatVector
        );
    }
}
