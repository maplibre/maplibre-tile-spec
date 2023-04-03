
export enum GeometryType{
    Point = 1,
    LineString = 2,
    Polygon = 3,
    MultiPoint = 4,
    MultiLineString = 5,
    MultiPolygon = 6
}

export interface Point{
    x: number,
    y: number
}

export type LineString = Point[];

export type LinearRing = LineString;

export type Polygon = LinearRing[];

export type MultiLineString = LineString[];

export interface LineStringTopology {
    numVertices: number;
}

export interface PolygonTopology {
    numVertices: number[];
}

export interface MultiLineStringTopology {
    numVertices: number[];
}

//TODO: use conditional types for Point vs Point[]
export interface Geometry{
    type: GeometryType,
    //TODO: refactor -> topology not needed as a result
    topology?: LineStringTopology | PolygonTopology | MultiLineStringTopology,
    vertices: Point | LineString | Polygon | MultiLineString
}

export type BoundingBox = [number, number, number, number];


