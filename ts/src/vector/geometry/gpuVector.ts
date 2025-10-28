import Point from "@mapbox/point-geometry";
import { type SelectionVector } from "../filter/selectionVector";
import { type SINGLE_PART_GEOMETRY_TYPE, GEOMETRY_TYPE } from "./geometryType";
import { type CoordinatesArray } from "./geometryVector";
import type TopologyVector from "./topologyVector";

export abstract class GpuVector implements Iterable<CoordinatesArray> {
    protected constructor(
        private readonly _triangleOffsets: Int32Array,
        private readonly _indexBuffer: Int32Array,
        private readonly _vertexBuffer: Int32Array,
        private readonly _topologyVector?: TopologyVector | null,
    ) {}

    abstract geometryType(index: number): number;

    abstract get numGeometries(): number;

    abstract filter(geometryType: SINGLE_PART_GEOMETRY_TYPE): SelectionVector;

    abstract filterSelected(geometryType: SINGLE_PART_GEOMETRY_TYPE, selectionVector: SelectionVector);

    abstract containsSingleGeometryType(): boolean;

    get triangleOffsets(): Int32Array {
        return this._triangleOffsets;
    }

    get indexBuffer(): Int32Array {
        return this._indexBuffer;
    }

    get vertexBuffer(): Int32Array {
        return this._vertexBuffer;
    }

    get topologyVector(): TopologyVector | null {
        return this._topologyVector;
    }

    /**
     * Returns geometries as coordinate arrays by extracting polygon outlines from topology.
     * The vertexBuffer contains the outline vertices, separate from the tessellated triangles.
     */
    getGeometries(): CoordinatesArray[] {
        if (!this._topologyVector) {
            throw new Error("Cannot convert GpuVector to coordinates without topology information");
        }

        const geometries: CoordinatesArray[] = new Array(this.numGeometries);
        const topology = this._topologyVector;
        const partOffsets = topology.partOffsets;
        const ringOffsets = topology.ringOffsets;
        const geometryOffsets = topology.geometryOffsets;

        // Use counters to track position in offset arrays (like Java implementation)
        let vertexBufferOffset = 0;
        let partOffsetCounter = 1;
        let ringOffsetsCounter = 1;
        let geometryOffsetsCounter = 1;

        for (let i = 0; i < this.numGeometries; i++) {
            const geometryType = this.geometryType(i);

            switch (geometryType) {
                case GEOMETRY_TYPE.POLYGON:
                    {
                        // Get number of rings for this polygon
                        const numRings = partOffsets[partOffsetCounter] - partOffsets[partOffsetCounter - 1];
                        partOffsetCounter++;
                        const rings: Point[][] = [];

                        for (let j = 0; j < numRings; j++) {
                            // Get number of vertices in this ring
                            const numVertices = ringOffsets[ringOffsetsCounter] - ringOffsets[ringOffsetsCounter - 1];
                            ringOffsetsCounter++;
                            const ring: Point[] = [];

                            for (let k = 0; k < numVertices; k++) {
                                const x = this._vertexBuffer[vertexBufferOffset++];
                                const y = this._vertexBuffer[vertexBufferOffset++];
                                ring.push(new Point(x, y));
                            }
                            // Close the ring by duplicating the first vertex (MVT format requirement)
                            if (ring.length > 0) {
                                ring.push(ring[0]);
                            }
                            rings.push(ring);
                        }

                        geometries[i] = rings;
                        if (geometryOffsets) geometryOffsetsCounter++;
                    }
                    break;
                case GEOMETRY_TYPE.MULTIPOLYGON:
                    {
                        // Get number of polygons in this multipolygon
                        const numPolygons =
                            geometryOffsets[geometryOffsetsCounter] - geometryOffsets[geometryOffsetsCounter - 1];
                        geometryOffsetsCounter++;
                        const allRings: Point[][] = [];

                        for (let p = 0; p < numPolygons; p++) {
                            // Get number of rings in this polygon
                            const numRings = partOffsets[partOffsetCounter] - partOffsets[partOffsetCounter - 1];
                            partOffsetCounter++;

                            for (let j = 0; j < numRings; j++) {
                                // Get number of vertices in this ring
                                const numVertices =
                                    ringOffsets[ringOffsetsCounter] - ringOffsets[ringOffsetsCounter - 1];
                                ringOffsetsCounter++;
                                const ring: Point[] = [];

                                for (let k = 0; k < numVertices; k++) {
                                    const x = this._vertexBuffer[vertexBufferOffset++];
                                    const y = this._vertexBuffer[vertexBufferOffset++];
                                    ring.push(new Point(x, y));
                                }
                                // Close the ring by duplicating the first vertex (MVT format requirement)
                                if (ring.length > 0) {
                                    ring.push(ring[0]);
                                }
                                allRings.push(ring);
                            }
                        }

                        geometries[i] = allRings;
                    }
                    break;
            }
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
