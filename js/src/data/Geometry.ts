import Point = require("@mapbox/point-geometry");
import { project } from './Projection';

function classifyRings(rings) {
    var len = rings.length;

    if (len <= 1) return [rings];

    var polygons = [],
        polygon,
        ccw;

    for (var i = 0; i < len; i++) {
        var area = signedArea(rings[i]);
        if (area === 0) continue;

        if (ccw === undefined) ccw = area < 0;

        if (ccw === area < 0) {
            if (polygon) polygons.push(polygon);
            polygon = [rings[i]];

        } else {
            polygon.push(rings[i]);
        }
    }
    if (polygon) polygons.push(polygon);

    return polygons;
}

function signedArea(ring) {
    var sum = 0;
    for (var i = 0, len = ring.length, j = len - 1, p1, p2; i < len; j = i++) {
        p1 = ring[i];
        p2 = ring[j];
        sum += (p2.x - p1.x) * (p1.y + p2.y);
    }
    return sum;
}

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
            const rings = [project(x, y, z, this.shell.points)];
            this.rings.forEach(ring => {
                rings.push(project(x, y, z, ring.points));
            });
            return {
                "type": "Polygon",
                "coordinates": rings
            };
        }
        return {
            "type": "Polygon",
            "coordinates": [project(x, y, z, this.shell.points)]
        };
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
    public toGeoJSON = (x: number, y: number, z: number) => {
        const lines = [];
        for (const line of this.lines) {
            lines.push(project(x, y, z, line.points));
        }
        return {
            "type": "MultiLineString",
            "coordinates": lines
        };
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

    public _toGeoJSON = (x: number, y: number, z: number) => {
        let coords = classifyRings(this.loadGeometry());
        for (let i = 0; i < coords.length; i++) {
            for (let j = 0; j < coords[i].length; j++) {
                coords[i][j] = project(x, y, z, coords[i][j]);
            }
        }
        let type = 'Polygon';
        if (coords.length === 1) {
            coords = coords[0];
        } else {
            type = 'Multi' + type;
        }
        return {
            "type": "MultiPolygon",
            "coordinates": coords
        };
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


