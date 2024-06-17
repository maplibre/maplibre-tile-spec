import Point = require("@mapbox/point-geometry");
import { project } from './Projection';

export class LineString {
    points : Point[];
    constructor(points: Point[]) {
        this.points = points;
    }
    public toGeoJSON = (x: number, y: number, z: number) => {
        return {
            "type": "LineString",
            "coordinates": project(x, y, z, this.points)
        };
    }
    public loadGeometry = () => {
        return [this.points];
    }
}

export class MultiPoint {
    points : Point[];
    constructor(points: Point[]) {
        this.points = points;
    }
    public toGeoJSON = (x: number, y: number, z: number) => {
        return {
            "type": "MultiPoint",
            "coordinates": project(x, y, z, this.points)
        };
    }
    public loadGeometry = () => {
        return this.points.map(p => [p]);
    }
}

export class LinearRing {
    points : Point[];
    constructor(points: Point[]) {
        this.points = points;
    }
    public toGeoJSON = (x: number, y: number, z: number) => {
        return {
            "type": "LineString",
            "coordinates": project(x, y, z, this.points)
        };
    }
    public loadGeometry = () => {
        return [this.points];
    }
}

export class Polygon {
    shell : LinearRing;
    rings : LinearRing[];
    constructor(shell: LinearRing, rings: LinearRing[]) {
        this.shell = shell;
        this.rings = rings;
    }
    public toGeoJSON = (x: number, y: number, z: number) => {
        if (this.rings.length) {
            throw new Error("Not implemented");
        }
        return {
            "type": "Polygon",
            "coordinates": [project(x, y, z, this.shell.points)]
        };
    }
    public loadGeometry = () => {
        if (this.rings.length) {
            throw new Error("Not implemented");
        }
        return this.shell.loadGeometry();
    }
}

export class MultiLineString {
    lines : LineString[];
    constructor(lineStrings: LineString[]) {
        this.lines = lineStrings;
    }
    public toGeoJSON = (x: number, y: number, z: number) => {
        throw new Error("Not implemented");
    }
    public loadGeometry = () => {
        throw new Error("Not implemented");
    }
}
export class MultiPolygon {
    polygons : Polygon[];
    constructor(polygons: Polygon[]) {
        this.polygons = polygons;
    }
    public toGeoJSON = (x: number, y: number, z: number) => {
        const polygons = [];
        for (const polygon of this.polygons) {
            const poly = [project(x, y, z, polygon.shell.points)];
            if (polygon.rings.length) {
                polygon.rings.forEach(ring => {
                    poly.push(project(x, y, z, ring.points));
                });
            }
            polygons.push(poly);
        }
        return {
            "type": "MultiPolygon",
            "coordinates": polygons
        };
    }

    public loadGeometry = () => {
        const polygons = [];
        for (const polygon of this.polygons) {
            polygons.push(polygon.shell.points);
            polygon.rings.forEach(ring => {
                polygons.push(ring.points);
            });
        }
        return polygons;
    }
}

export class GeometryFactory {
    createPoint(x: number, y:number) {
        return new Point(x, y);
    }
    createMultiPoint(points: Point[]) {
        return new MultiPoint(points);
    }
    createLineString(vertices: Point[]) {
        return new LineString(vertices);
    }
    createLinearRing(linearRing: Point[]): LinearRing {
        return new LinearRing(linearRing);
    }
    createPolygon(shell: LinearRing, rings: LinearRing[]) {
        return new Polygon(shell, rings);
    }
    createMultiLineString(lineStrings: LineString[]) {
        return new MultiLineString(lineStrings);
    }
    createMultiPolygon(polygons: Polygon[]) {
        return new MultiPolygon(polygons);
    }
}


