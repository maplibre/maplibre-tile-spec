import { type Geometry, type GeometryVector } from "./geometry/geometryVector";
import type Vector from "./vector";
import { type IntVector } from "./intVector";
import { IntFlatVector } from "./flat/intFlatVector";
import { DoubleFlatVector } from "./flat/doubleFlatVector";
import { IntSequenceVector } from "./sequence/intSequenceVector";
import { IntConstVector } from "./constant/intConstVector";
import { type GpuVector } from "./geometry/gpuVector";

export interface Feature {
    id: number | bigint;
    geometry: Geometry;
    properties: { [key: string]: unknown };
}

export default class FeatureTable implements Iterable<Feature> {
    private propertyVectorsMap: Map<string, Vector>;

    constructor(
        private readonly _name: string,
        private readonly _geometryVector: GeometryVector | GpuVector,
        private readonly _idVector?: IntVector,
        private readonly _propertyVectors?: Vector[],
        private readonly _extent = 4096,
    ) {}

    get name(): string {
        return this._name;
    }

    get idVector(): IntVector {
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

    *[Symbol.iterator](): Iterator<Feature> {
        const geometryIterator = this.geometryVector[Symbol.iterator]();
        let index = 0;

        while (index < this.numFeatures) {
            let id;
            if (this.idVector) {
                id = this.containsMaxSaveIntegerValues(this.idVector)
                    ? Number(this.idVector.getValue(index))
                    : this.idVector.getValue(index);
            }
            const geometry = geometryIterator?.next().value;

            const properties: { [key: string]: unknown } = {};
            for (const propertyColumn of this.propertyVectors) {
                if (!propertyColumn) {
                    continue;
                }

                const columnName = propertyColumn.name;
                const propertyValue = propertyColumn.getValue(index);
                if (propertyValue !== null) {
                    properties[columnName] = propertyValue;
                }
            }

            index++;
            yield { id, geometry, properties };
        }
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
                id = this.containsMaxSaveIntegerValues(this.idVector) && idValue !== null ? Number(idValue) : idValue;
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

    private containsMaxSaveIntegerValues(intVector: IntVector) {
        return (
            intVector instanceof IntFlatVector ||
            (intVector instanceof IntConstVector && intVector instanceof IntSequenceVector) ||
            intVector instanceof DoubleFlatVector
        );
    }
}
