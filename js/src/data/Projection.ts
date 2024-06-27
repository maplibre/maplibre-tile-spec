

// Intended to match the results of
// https://github.com/mapbox/vector-tile-js/blob/77851380b63b07fd0af3d5a3f144cc86fb39fdd1/lib/vectortilefeature.js#L129

export class Projection {
    size: number;
    x0: number;
    y0: number;
    s1: number;
    s2: number;

    constructor(extent: number, x: number, y: number, z: number) {
        this.size = extent * Math.pow(2, z);
        this.x0 = extent * x;
        this.y0 = extent * y;
        this.s1 = 360 / this.size;
        this.s2 = 360 / Math.PI;
    }

    public project(points) {
        const projected = new Array(points.length);
        for (let j = 0; j < points.length; j++) {
            const point = points[j];
            const y2 = 180 - (point.y + this.y0) * this.s1;
            projected[j] = [
                (point.x + this.x0) * 360 / this.size - 180,
                this.s2 * Math.atan(Math.exp(y2 * Math.PI / 180)) - 90
            ];
        }
        return projected;
    }
}
