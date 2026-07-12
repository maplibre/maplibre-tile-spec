import Point from "@mapbox/point-geometry";
import { GEOMETRY_TYPE } from "./geometryType";
import type { CoordinatesArray } from "./geometryVector";
import type { TopologyVector } from "./topologyVector";

export type RandomAccessGeometryVector = {
    readonly numGeometries: number;
    readonly topologyVector: TopologyVector;
    readonly containsPolygonGeometry: boolean;
    geometryType(index: number): number;
    getVertex(index: number): [number, number];
};

type GeometryCursor = {
    vertex: number;
    geometry: number;
    part: number;
    ring: number;
};

const CHECKPOINT_INTERVAL = 64;

export class PerFeatureGeometryReader {
    private readonly checkpoints: GeometryCursor[] = [{ vertex: 0, geometry: 1, part: 1, ring: 1 }];
    private sequentialCursor?: { index: number; cursor: GeometryCursor };

    constructor(private readonly geometryVector: RandomAccessGeometryVector) {}

    private advanceCursor(cursor: GeometryCursor, index: number): GeometryCursor {
        const { geometryOffsets, partOffsets, ringOffsets } = this.geometryVector.topologyVector;
        const next = cursor;

        switch (this.geometryVector.geometryType(index)) {
            case GEOMETRY_TYPE.POINT:
                next.vertex++;
                if (geometryOffsets) next.geometry++;
                if (partOffsets) next.part++;
                if (ringOffsets) next.ring++;
                break;
            case GEOMETRY_TYPE.MULTIPOINT: {
                const count = offsetLength(geometryOffsets, next.geometry++);
                next.vertex += count;
                next.part += count;
                next.ring += count;
                break;
            }
            case GEOMETRY_TYPE.LINESTRING:
                next.vertex += this.geometryVector.containsPolygonGeometry
                    ? offsetLength(ringOffsets, next.ring++)
                    : offsetLength(partOffsets, next.part);
                next.part++;
                if (geometryOffsets) next.geometry++;
                break;
            case GEOMETRY_TYPE.POLYGON: {
                const ringCount = offsetLength(partOffsets, next.part++);
                for (let j = 0; j < ringCount; j++) next.vertex += offsetLength(ringOffsets, next.ring++);
                if (geometryOffsets) next.geometry++;
                break;
            }
            case GEOMETRY_TYPE.MULTILINESTRING: {
                const partCount = offsetLength(geometryOffsets, next.geometry++);
                for (let j = 0; j < partCount; j++) {
                    next.vertex += this.geometryVector.containsPolygonGeometry
                        ? offsetLength(ringOffsets, next.ring++)
                        : offsetLength(partOffsets, next.part);
                    next.part++;
                }
                break;
            }
            case GEOMETRY_TYPE.MULTIPOLYGON: {
                const polygonCount = offsetLength(geometryOffsets, next.geometry++);
                for (let j = 0; j < polygonCount; j++) {
                    const ringCount = offsetLength(partOffsets, next.part++);
                    for (let k = 0; k < ringCount; k++) next.vertex += offsetLength(ringOffsets, next.ring++);
                }
                break;
            }
            default:
                throw new Error(
                    `The specified geometry type (${this.geometryVector.geometryType(index)}) is currently not supported.`,
                );
        }
        return next;
    }

    private getCursor(index: number): GeometryCursor {
        if (this.sequentialCursor?.index === index) return { ...this.sequentialCursor.cursor };

        const checkpointIndex = Math.floor(index / CHECKPOINT_INTERVAL);
        while (this.checkpoints.length <= checkpointIndex) {
            let cursor = { ...this.checkpoints[this.checkpoints.length - 1] };
            const start = (this.checkpoints.length - 1) * CHECKPOINT_INTERVAL;
            const end = Math.min(start + CHECKPOINT_INTERVAL, this.geometryVector.numGeometries);
            for (let i = start; i < end; i++) cursor = this.advanceCursor(cursor, i);
            this.checkpoints.push(cursor);
        }

        let cursor = { ...this.checkpoints[checkpointIndex] };
        for (let i = checkpointIndex * CHECKPOINT_INTERVAL; i < index; i++) {
            cursor = this.advanceCursor(cursor, i);
        }
        return cursor;
    }

    getGeometry(index: number): CoordinatesArray {
        if (!Number.isInteger(index) || index < 0 || index >= this.geometryVector.numGeometries) {
            throw new RangeError(`Geometry index ${index} is out of bounds`);
        }

        const geometryType = this.geometryVector.geometryType(index);
        const { geometryOffsets, partOffsets, ringOffsets } = this.geometryVector.topologyVector;
        const cursor = this.getCursor(index);
        let { vertex, part, ring } = cursor;
        const { geometry } = cursor;
        const result: CoordinatesArray = [];

        const addVertices = (count: number, closeRing: boolean) => {
            const points = new Array<Point>(count + (closeRing ? 1 : 0));
            for (let i = 0; i < count; i++) {
                const [x, y] = this.geometryVector.getVertex(vertex++);
                points[i] = new Point(x, y);
            }
            if (closeRing && count > 0) points[count] = new Point(points[0].x, points[0].y);
            result.push(points);
        };

        switch (geometryType) {
            case GEOMETRY_TYPE.POINT:
                addVertices(1, false);
                break;
            case GEOMETRY_TYPE.MULTIPOINT: {
                const count = offsetLength(geometryOffsets, geometry);
                for (let i = 0; i < count; i++) addVertices(1, false);
                break;
            }
            case GEOMETRY_TYPE.LINESTRING:
                addVertices(
                    this.geometryVector.containsPolygonGeometry
                        ? offsetLength(ringOffsets, ring)
                        : offsetLength(partOffsets, part),
                    false,
                );
                break;
            case GEOMETRY_TYPE.POLYGON: {
                const ringCount = offsetLength(partOffsets, part);
                for (let i = 0; i < ringCount; i++) addVertices(offsetLength(ringOffsets, ring++), true);
                break;
            }
            case GEOMETRY_TYPE.MULTILINESTRING: {
                const partCount = offsetLength(geometryOffsets, geometry);
                for (let i = 0; i < partCount; i++) {
                    const count = this.geometryVector.containsPolygonGeometry
                        ? offsetLength(ringOffsets, ring++)
                        : offsetLength(partOffsets, part);
                    part++;
                    addVertices(count, false);
                }
                break;
            }
            case GEOMETRY_TYPE.MULTIPOLYGON: {
                const polygonCount = offsetLength(geometryOffsets, geometry);
                for (let i = 0; i < polygonCount; i++) {
                    const ringCount = offsetLength(partOffsets, part++);
                    for (let j = 0; j < ringCount; j++) addVertices(offsetLength(ringOffsets, ring++), true);
                }
                break;
            }
            default:
                throw new Error(`The specified geometry type (${geometryType}) is currently not supported.`);
        }

        this.sequentialCursor = { index: index + 1, cursor: this.advanceCursor(cursor, index) };
        return result;
    }
}

function offsetLength(offsets: Uint32Array | undefined, index: number): number {
    if (!offsets) throw new Error("Invalid MLT geometry topology");
    return offsets[index] - offsets[index - 1];
}
