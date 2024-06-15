import { Geometry } from './Geometry';

export class Feature {
    id: number;
    geometry: Geometry;
    /* eslint-disable @typescript-eslint/no-explicit-any */
    properties: { [key: string]: any };
    constructor(id: number, geometry : Geometry, properties: { [key: string]: any }) {
        this.id = id;
        this.geometry = geometry;
        this.properties = properties;
    }
    public toGeoJSON = () : object => {
        return {
            "type": "Feature",
            "geometry": this.geometry.toGeoJSON(),
            "properties": this.properties
        };
    }
}
