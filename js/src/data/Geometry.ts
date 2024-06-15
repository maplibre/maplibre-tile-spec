export abstract class Geometry {
    coordinates: any[];
    public toGeoJSON = () : object => {
        return {};
    }
}

export class Point extends Geometry {
    constructor(coordinate: Coordinate) {
        super();
        this.coordinates = [coordinate.x, coordinate.y];
    }
    public toGeoJSON = () : object => {
        return {
            "type": "Point",
            "coordinates": this.coordinates
        };
    }
}

export class LineString extends Geometry {
    constructor(vertices: Coordinate[]) {
        super();
        this.coordinates = [];
        for (const vertex of vertices) {
            this.coordinates.push([vertex.x, vertex.y]);
        }
    }
    public toGeoJSON = () : object => {
        return {
            "type": "LineString",
            "coordinates": this.coordinates
        };
    }
}

// TODO: rings not quite nested correctly or closed yet
export class Polygon extends Geometry {
    constructor(shell: LinearRing, rings: LinearRing[]) {
        super();
        this.coordinates = [ [shell.coordinates] ];
        if (rings.length > 0) {
            this.coordinates.push([]);
            const rings = this.coordinates[1];
            for (const ring of rings) {
                rings.push(ring.coordinates);
            }
        }
    }
    public toGeoJSON = () : object => {
        return {
            "type": "Polygon",
            "coordinates": this.coordinates
        };
    }
}

export class LinearRing extends Geometry {
    constructor(linearRing: Coordinate[]) {
        super();
        this.coordinates = [];
        for (const vertex of linearRing) {
            this.coordinates.push(vertex.x);
            this.coordinates.push(vertex.y);
        }
    }
    public toGeoJSON = () : object => {
        return this.coordinates;
    }
}

export class MultiPoint extends Geometry {
    constructor(points: Point[]) {
        super();
        this.coordinates = [];
        for (const point of points) {
            this.coordinates.push(point.coordinates);
        }
    }
    public toGeoJSON = () : object => {
        return {
            "type": "MultiPoint",
            "coordinates": this.coordinates
        };
    }
}

export class MultiLineString extends Geometry {
    constructor(lineStrings: LineString[]) {
        super();
        this.coordinates = [];
        for (const lineString of lineStrings) {
            this.coordinates.push(lineString.coordinates);
        }
    }
    public toGeoJSON = () : object => {
        return {
            "type": "MultiLineString",
            "coordinates": this.coordinates
        };
    }
}

export class MultiPolygon extends Geometry {
    constructor(polygons: Polygon[]) {
        super();
        this.coordinates = [];
        for (const polygon of polygons) {
            this.coordinates.push(polygon.coordinates);
        }
    }
    public toGeoJSON = () : object => {
        return {
            "type": "MultiPolygon",
            "coordinates": this.coordinates
        };
    }
}

export class Coordinate {
    constructor(x: number, y: number) {
        this.x = x;
        this.y = y;
    }
    x: number;
    y: number;

    public toGeoJSON = () : object => {
        return [ this.x, this.y] ;
    }
}

export class GeometryFactory {
    createPoint(coordinate: Coordinate): Geometry {
        return new Point(coordinate);
    }
    createMultiPoint(points: Point[]): Geometry {
        return new MultiPoint(points);
    }
    createLineString(vertices: Coordinate[]): Geometry {
        return new LineString(vertices);
    }
    createPolygon(shell: LinearRing, rings: LinearRing[]): Geometry {
        return new Polygon(shell, rings);
    }
    createMultiLineString(lineStrings: LineString[]): Geometry {
        return new MultiLineString(lineStrings);
    }
    createMultiPolygon(polygons: Polygon[]): Geometry {
        return new MultiPolygon(polygons);
    }
    createLinearRing(linearRing: Coordinate[]): LinearRing {
        return new LinearRing(linearRing);
    }
}

export enum GeometryType {
    POINT,
    LINESTRING,
    POLYGON,
    MULTIPOINT,
    MULTILINESTRING,
    MULTIPOLYGON
}
