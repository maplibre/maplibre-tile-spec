import { Feature } from './Feature';

export class Layer {
    name: string;
    version: number;
    length: number;
    features: Feature[];

    constructor(name: string, version : number, features: Feature[]) {
        this.name = name;
        this.length = features.length;
        this.features = features;
        this.version = version;
    }

    public feature = (i) => {
      if (i < 0 || i >= this.length) throw new Error('feature index out of bounds');
      return this.features[i];
    }
}
