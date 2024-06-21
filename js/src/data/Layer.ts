import { Feature } from './Feature';

export class Layer {
    name: string;
    version: number;
    features: Feature[];

    constructor(name: string, version : number, features: Feature[]) {
        this.name = name;
        this.features = features;
        this.version = version;
    }
}
