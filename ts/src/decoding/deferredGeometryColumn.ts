import { decodeGeometryColumn } from "./geometryDecoder";
import IntWrapper from "./intWrapper";
import type GeometryScaling from "./geometryScaling";
import type { GeometryVector } from "../vector/geometry/geometryVector";
import type { GpuVector } from "../vector/geometry/gpuVector";

type GeometryColumnArgs = {
    tile: Uint8Array;
    numStreams: number;
    numFeatures: number;
    scalingData?: GeometryScaling;
    startOffset: number;
};

export class DeferredGeometryColumn {
    private decoded: GeometryVector | GpuVector | null = null;

    constructor(private readonly args: GeometryColumnArgs) {}

    get numFeatures(): number {
        return this.args.numFeatures;
    }

    get(): GeometryVector | GpuVector {
        if (!this.decoded) {
            const offset = new IntWrapper(this.args.startOffset);
            this.decoded = decodeGeometryColumn(
                this.args.tile,
                this.args.numStreams,
                offset,
                this.args.numFeatures,
                this.args.scalingData,
            );
        }

        return this.decoded;
    }
}
