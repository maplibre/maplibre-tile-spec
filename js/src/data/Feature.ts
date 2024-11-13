import { Projection } from './Projection';
import { GeometryType } from './GeometryType';

export class Feature {
    id: number;
    extent: number;
    geometry;
    properties;
    type: GeometryType;
    constructor(id: number, extent: number, geometry, properties) {
        this.id = id;
        this.geometry = geometry;
        this.properties = properties;
        this.extent = extent;
        this.type = geometry.type ? geometry.type() : GeometryType.Point;
    }

    public loadGeometry = () => {
        if (typeof this.geometry.loadGeometry === 'function') {
            return this.geometry.loadGeometry();
        } else {
            return [[this.geometry]];
        }
    }

    public toGeoJSON = (x: number, y: number, z: number) => {
        let geometry;
        if (typeof this.geometry.toGeoJSON === 'function') {
            geometry = this.geometry.toGeoJSON(this.extent, x, y, z);
        } else {
            const projection = new Projection(this.extent, x, y, z);
            const projected = projection.project([this.geometry]);
            geometry = {
                "type": "Point",
                "coordinates": projected[0]
            };
        }
        return {
            type: "Feature",
            id: Number(this.id),
            geometry: geometry,
            properties: this.properties
        };
    }
}
