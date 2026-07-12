import Point from "@mapbox/point-geometry";
import type { CoordinatesArray } from "./geometryVector";
import { GEOMETRY_TYPE } from "./geometryType";
import type { TopologyVector } from "./topologyVector";
import { PerFeatureGeometryReader } from "./perFeatureGeometryReader";

export abstract class GpuVector implements Iterable<CoordinatesArray> {
    private geometryReader?: PerFeatureGeometryReader;

    protected constructor(
        private readonly _triangleOffsets: Uint32Array,
        private readonly _indexBuffer: Uint32Array,
        private readonly _vertexBuffer: Int32Array | Uint32Array,
        private readonly _topologyVector?: TopologyVector,
    ) {}

    abstract geometryType(index: number): number;

    abstract get numGeometries(): number;

    abstract containsSingleGeometryType(): boolean;

    get triangleOffsets(): Uint32Array {
        return this._triangleOffsets;
    }

    get indexBuffer(): Uint32Array {
        return this._indexBuffer;
    }

    get vertexBuffer(): Int32Array | Uint32Array {
        return this._vertexBuffer;
    }

    get topologyVector(): TopologyVector | undefined {
        return this._topologyVector;
    }

    getGeometry(index: number): CoordinatesArray {
        if (!this.topologyVector) {
            throw new Error("Cannot access GpuVector geometry without topology information");
        }
        this.geometryReader ??= new PerFeatureGeometryReader({
            numGeometries: this.numGeometries,
            topologyVector: this.topologyVector,
            containsPolygonGeometry: true,
            geometryType: (geometryIndex) => this.geometryType(geometryIndex),
            getVertex: (vertexIndex) => {
                const offset = vertexIndex * 2;
                return [this.vertexBuffer[offset], this.vertexBuffer[offset + 1]];
            },
        });
        return this.geometryReader.getGeometry(index);
    }

    /**
     * Returns geometries as coordinate arrays by extracting polygon outlines from topology.
     * The vertexBuffer contains the outline vertices, separate from the tessellated triangles.
     */
    getGeometries(): CoordinatesArray[] {
        if (!this.topologyVector) {
            throw new Error("Cannot convert GpuVector to coordinates without topology information");
        }

        const { geometryOffsets, partOffsets, ringOffsets } = this.topologyVector;
        const geometries = new Array<CoordinatesArray>(this.numGeometries);
        let vertex = 0;
        let geometry = 1;
        let part = 1;
        let ring = 1;

        const readRing = (): Point[] => {
            const count = gpuOffsetLength(ringOffsets, ring++);
            const points = new Array<Point>(count + (count > 0 ? 1 : 0));
            for (let i = 0; i < count; i++) {
                points[i] = new Point(this.vertexBuffer[vertex++], this.vertexBuffer[vertex++]);
            }
            if (count > 0) points[count] = points[0];
            return points;
        };

        for (let i = 0; i < this.numGeometries; i++) {
            const result: CoordinatesArray = [];
            switch (this.geometryType(i)) {
                case GEOMETRY_TYPE.POLYGON: {
                    const ringCount = gpuOffsetLength(partOffsets, part++);
                    for (let j = 0; j < ringCount; j++) result.push(readRing());
                    if (geometryOffsets) geometry++;
                    break;
                }
                case GEOMETRY_TYPE.MULTIPOLYGON: {
                    const polygonCount = gpuOffsetLength(geometryOffsets, geometry++);
                    for (let j = 0; j < polygonCount; j++) {
                        const ringCount = gpuOffsetLength(partOffsets, part++);
                        for (let k = 0; k < ringCount; k++) result.push(readRing());
                    }
                    break;
                }
                default:
                    throw new Error(`GPU geometry type ${this.geometryType(i)} is not supported`);
            }
            geometries[i] = result;
        }
        return geometries;
    }

    [Symbol.iterator](): Iterator<CoordinatesArray> {
        /*for(let i = 1; i < this.triangleOffsets.length; i++) {
           const numTriangles = this.triangleOffsets[i] - this.triangleOffsets[i-1];
           const startIndex = this.triangleOffsets[i-1] * 3;
           const endIndex = this.triangleOffsets[i] * 3;
       }

        while (index < this.numGeometries) {
            yield geometries[index++];
        }*/

        //throw new Error("Iterator on a GpuVector is not implemented yet.");
        return null;
    }
}

function gpuOffsetLength(offsets: Uint32Array | undefined, index: number): number {
    if (!offsets) throw new Error("Invalid GPU geometry topology");
    return offsets[index] - offsets[index - 1];
}
