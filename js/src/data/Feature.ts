export class Feature {
    id: number;
    geometry: string;
    /* eslint-disable @typescript-eslint/no-explicit-any */
    properties: { [key: string]: any };
    // TODO: Add geometry type
    constructor(id: number, geometry, properties: { [key: string]: any }) {
        this.id = id;
        this.geometry = geometry;
        this.properties = properties;
    }
}
