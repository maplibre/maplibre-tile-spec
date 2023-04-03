import { GeometryType } from "./geometry";

export interface MVTFeature {
    id: number;
    type: number;
    geometry: { x: number; y: number }[][];
    properties: { [name: string]: any };
    hilbertIndices?: number[];
}

export interface MVTLayer {
    type: GeometryType;
    features: MVTFeature[];
}
