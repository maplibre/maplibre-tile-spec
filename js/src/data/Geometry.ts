export class Geometry {}
export class Point extends Geometry {}
export class LineString extends Geometry {}
export class Polygon extends Geometry {}
export class LinearRing extends Geometry {}

export class Coordinate {
    constructor(x: number, y: number) {
        this.x = x;
        this.y = y;
    }
    public toString = () : string => {
        return `[` + this.x + ',' + this.y + ']';
    }
    x: number;
    y: number;
}

export enum GeometryType {
    POINT,
    LINESTRING,
    POLYGON,
    MULTIPOINT,
    MULTILINESTRING,
    MULTIPOLYGON
}

// export enum GeometryType {
//     POINT = "POINT",
//     MULTIPOINT = "MULTIPOINT",
//     LINESTRING = "LINESTRING",
//     POLYGON = "POLYGON",
//     MULTILINESTRING = "MULTILINESTRING",
//     MULTIPOLYGON = "MULTIPOLYGON"
// }
