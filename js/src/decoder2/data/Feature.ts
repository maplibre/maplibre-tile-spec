// import { Geometry } from 'geojson';

export class Feature {
    id: number;
    geometry: string;
    properties: { [key: string]: any };

    constructor(id: number, geometry: string, properties: { [key: string]: any }) {
        this.id = id;
        this.geometry = geometry;
        this.properties = properties;
    }
}
