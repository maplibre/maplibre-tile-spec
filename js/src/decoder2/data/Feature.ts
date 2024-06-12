export class Feature {
    id: number;
    geometry: string;
    properties: { [key: string]: any };
    // TODO: Add geometry type
    constructor(id: number, geometry, properties: { [key: string]: any }) {
        this.id = id;
        this.geometry = geometry;
        this.properties = properties;
    }
}
