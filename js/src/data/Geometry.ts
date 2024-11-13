import Point = require("@mapbox/point-geometry");
import { Projection } from './Projection';
import { GeometryType } from './GeometryType';

export class Coordinate {
    x: number;
    y: number;
    constructor(x: number, y: number) {
        this.x = x;
        this.y = y;
    }
}

export class LineString {
    points : Point[];
    constructor(points: Point[]) {
        this.points = points;
    }
    public toGeoJSON = (extent: number, x: number, y: number, z: number) => {
        const projection = new Projection(extent, x, y, z);
        return {
            "type": "LineString",
            "coordinates": projection.project(this.points)
        };
    }
    public type = () : GeometryType => {
        return GeometryType.LineString;
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
    public toGeoJSON = (extent: number, x: number, y: number, z: number) => {
        const projection = new Projection(extent, x, y, z);
        return {
            "type": "MultiPoint",
            "coordinates": projection.project(this.points)
        };
    }
    public type = () : GeometryType => {
        return GeometryType.Point;
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
    public toGeoJSON = (extent: number, x: number, y: number, z: number) => {
        const projection = new Projection(extent, x, y, z);
        return {
            "type": "LineString",
            "coordinates": projection.project(this.points)
        };
    }
    public type = () : GeometryType => {
        return GeometryType.LineString;
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
    public toGeoJSON = (extent: number, x: number, y: number, z: number) => {
        const projection = new Projection(extent, x, y, z);
        if (this.rings.length) {
            const rings = [projection.project(this.shell.points)];
            this.rings.forEach(ring => {
                rings.push(projection.project(ring.points));
            });
            return {
                "type": "Polygon",
                "coordinates": rings
            };
        }
        return {
            "type": "Polygon",
            "coordinates": [projection.project(this.shell.points)]
        };
    }
    public type = () : GeometryType => {
        return GeometryType.Polygon;
    }
    public loadGeometry = () => {
        if (this.rings.length) {
            const rings = [this.shell.points];
            this.rings.forEach(ring => {
                rings.push(ring.points);
            });
            return rings;
        }
        return this.shell.loadGeometry();
    }
}

export class MultiLineString {
    lines : LineString[];
    constructor(lines: LineString[]) {
        this.lines = lines;
    }
    public toGeoJSON = (extent: number, x: number, y: number, z: number) => {
        const projection = new Projection(extent, x, y, z);
        const lines = [];
        for (const line of this.lines) {
            lines.push(projection.project(line.points));
        }
        return {
            "type": "MultiLineString",
            "coordinates": lines
        };
    }
    public type = () : GeometryType => {
        return GeometryType.LineString;
    }
    public loadGeometry = () => {
        const lines = [];
        for (const line of this.lines) {
            lines.push(line.points);
        }
        return lines;
    }
}
export class MultiPolygon {
    polygons : Polygon[];
    constructor(polygons: Polygon[]) {
        this.polygons = polygons;
    }

    public toGeoJSON = (extent: number, x: number, y: number, z: number) => {
        const projection = new Projection(extent, x, y, z);
        const polygons = [];
        for (const polygon of this.polygons) {
            const poly = [projection.project(polygon.shell.points)];
            if (polygon.rings.length) {
                polygon.rings.forEach(ring => {
                    poly.push(projection.project(ring.points));
                });
            }
            polygons.push(poly);
        }
        return {
            "type": "MultiPolygon",
            "coordinates": polygons
        };
    }

    public type = () : GeometryType => {
        return GeometryType.Polygon;
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
    createPoint(coordinate: Coordinate) {
        return new Point(coordinate.x, coordinate.y);
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


