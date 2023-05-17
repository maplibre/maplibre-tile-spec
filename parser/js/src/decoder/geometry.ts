export enum GeometryType {
    POINT,
    LINESTRING,
    POLYGON,
    MULTI_POINT,
    MULTI_LINESTRING,
    MULTI_POLYGON,
}

export type VertexBufferSlice = Int32Array;

export interface Vertex {
    x: number;
    y: number;
}

export abstract class Geometry {
    protected constructor(private readonly _type: GeometryType, private readonly _vertices: VertexBufferSlice) {}

    get type(): GeometryType {
        return this._type;
    }

    get vertices(): VertexBufferSlice {
        return this._vertices;
    }

    abstract format(): Array<Array<Vertex>>;
}

export class Point extends Geometry {
    constructor(_vertices: VertexBufferSlice) {
        super(GeometryType.POINT, _vertices);
    }

    format(): Array<Array<Vertex>> {
        const x = this.vertices[0];
        const y = this.vertices[1];
        return [[{ x, y }]];
    }
}

export class LineString extends Geometry {
    constructor(_vertices: VertexBufferSlice) {
        super(GeometryType.LINESTRING, _vertices);
    }

    format(): Array<Array<Vertex>> {
        const lineString = this.convertLineString();
        return [lineString];
    }

    private convertLineString() {
        const numVertices = this.vertices.length / 2;
        const lineString = new Array<Vertex>(numVertices);
        for (let i = 0; i < numVertices; i++) {
            const vertexOffset = i * 2;
            const x = this.vertices[vertexOffset];
            const y = this.vertices[vertexOffset + 1];
            lineString[i] = { x, y };
        }
        return lineString;
    }
}

export class IceLineString extends Geometry {
    constructor(_vertices: Int32Array, private readonly _vertexOffsets: Uint32Array) {
        super(GeometryType.LINESTRING, _vertices);
    }

    get vertexOffsets(): Uint32Array {
        return this._vertexOffsets;
    }

    format(): Array<Array<Vertex>> {
        const lineString = this.convertIceLineString();
        return [lineString];
    }

    private convertIceLineString() {
        const numVertices = this.vertexOffsets.length;
        const lineString = new Array<Vertex>(numVertices);
        for (let i = 0; i < numVertices; i++) {
            const vertexOffset = this.vertexOffsets[i] * 2;
            const x = this.vertices[vertexOffset];
            const y = this.vertices[vertexOffset + 1];
            lineString[i] = { x, y };
        }
        return lineString;
    }
}

export class Polygon extends Geometry {
    /**
     * @param _vertices Has to be transformed for usage in earcut currently -> extend for typed array support
     * @param _ringOffsets Can be directly used in earcut
     */
    constructor(_vertices: VertexBufferSlice, private readonly _ringOffsets: Uint32Array) {
        super(GeometryType.POLYGON, _vertices);
    }

    /*
     * Offsets of the rings in the VertexBuffer slice -> currently num vertices per ring
     * -> should be a slice with hole indices for directly use in earcut
     * */
    get ringOffsets(): Uint32Array {
        return this._ringOffsets;
    }

    format(): Array<Array<Vertex>> {
        const numRings = this.ringOffsets.length;
        const rings = new Array<Array<Vertex>>(numRings);
        let vertexOffset = 0;
        for (let j = 0; j < numRings; j++) {
            const numVertices = this.ringOffsets[j];
            const ring = new Array<Vertex>(numVertices + 1);
            for (let i = 0; i < numVertices; i++) {
                const x = this.vertices[vertexOffset++];
                const y = this.vertices[vertexOffset++];
                ring[i] = { x, y };
            }
            ring[ring.length - 1] = { x: ring[0].x, y: ring[0].y };
            rings[j] = ring;
        }

        return rings;
    }
}

export class MultiPoint extends Geometry {
    constructor(_vertices: VertexBufferSlice) {
        super(GeometryType.MULTI_POINT, _vertices);
    }

    format(): Array<Array<Vertex>> {
        const numPoints = this.vertices.length;
        const points = new Array<Vertex>(numPoints);
        for (let i = 0; i < numPoints; i++) {
            const vertexOffset = i * 2;
            const x = this.vertices[vertexOffset];
            const y = this.vertices[vertexOffset + 1];
            points[i] = { x, y };
        }
        return [points];
    }
}

export class MultiLineString extends Geometry {
    constructor(_vertices: VertexBufferSlice, private readonly _geometryOffsets: Uint32Array) {
        super(GeometryType.MULTI_LINESTRING, _vertices);
    }

    /*
     * GeometryOffsets slice -> currently num vertices per LineString
     * */
    get geometryOffsets(): Uint32Array {
        return this._geometryOffsets;
    }

    format(): Array<Array<Vertex>> {
        const numLineStrings = this.geometryOffsets.length;
        const lineStrings = new Array<Array<Vertex>>(numLineStrings);
        let vertexOffset = 0;
        for (let j = 0; j < numLineStrings; j++) {
            const numVertices = this.geometryOffsets[j];
            const lineString = new Array<Vertex>(numVertices);
            for (let i = 0; i < numVertices; i++) {
                const x = this.vertices[vertexOffset++];
                const y = this.vertices[vertexOffset++];
                lineString[i] = { x, y };
            }
            lineStrings[j] = lineString;
        }

        return lineStrings;
    }
}

export class IceMultiLineString extends Geometry {
    constructor(
        _vertices: VertexBufferSlice,
        private readonly _geometryOffsets: Uint32Array,
        private readonly _vertexOffsets: Uint32Array,
    ) {
        super(GeometryType.MULTI_LINESTRING, _vertices);
    }

    /*
     * GeometryOffsets slice -> currently num vertices per LineString
     * */
    get geometryOffsets(): Uint32Array {
        return this._geometryOffsets;
    }

    get vertexOffsets(): Uint32Array {
        return this._vertexOffsets;
    }

    format(): Array<Array<Vertex>> {
        const numLineStrings = this.geometryOffsets.length;
        const lineStrings = new Array<Array<Vertex>>(numLineStrings);
        let vertexOffsetsOffset = 0;
        for (let j = 0; j < numLineStrings; j++) {
            const numVertices = this.geometryOffsets[j];
            const lineString = new Array<Vertex>(numVertices);
            for (let i = 0; i < numVertices; i++) {
                const vertexOffset = this._vertexOffsets[vertexOffsetsOffset++] * 2;
                const x = this.vertices[vertexOffset];
                const y = this.vertices[vertexOffset + 1];
                lineString[i] = { x, y };
            }
            lineStrings[j] = lineString;
        }

        return lineStrings;
    }
}

export class MultiPolygon extends Geometry {
    constructor(
        _vertices: VertexBufferSlice,
        private readonly _partOffsets: Uint32Array,
        private readonly _ringOffsets: Uint32Array,
    ) {
        super(GeometryType.MULTI_POLYGON, _vertices);
    }

    /*
     * currently number of rings per Polygon
     * */
    get partOffsets(): Uint32Array {
        return this._partOffsets;
    }

    /*
     * currently number of vertices per LinearRing
     * */
    get ringOffsets(): Uint32Array {
        return this._ringOffsets;
    }

    format(): Array<Array<Vertex>> {
        const numGeometries = this._partOffsets.length;
        const polygons = new Array<Array<Vertex>>();
        let vertexBufferOffset = 0;
        let ringOffsetsOffset = 0;
        for (let k = 0; k < numGeometries; k++) {
            const numRings = this.partOffsets[k];
            for (let j = 0; j < numRings; j++) {
                const numVertices = this.ringOffsets[ringOffsetsOffset++];
                const ring = new Array<Vertex>(numVertices + 1);
                for (let i = 0; i < numVertices; i++) {
                    const x = this.vertices[vertexBufferOffset++];
                    const y = this.vertices[vertexBufferOffset++];
                    ring[i] = { x, y };
                }
                ring[ring.length - 1] = { x: ring[0].x, y: ring[0].y };
                polygons.push(ring);
            }
        }

        return polygons;
    }
}
