import { Feature } from './Feature';

export class Layer {
    name: string;
    features: Feature[];

    constructor(name: string, features: Feature[]) {
        this.name = name;
        this.features = features;
    }
}
