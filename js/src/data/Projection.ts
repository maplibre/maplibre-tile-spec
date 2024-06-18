

// Intended to match the results of
// https://github.com/mapbox/vector-tile-js/blob/77851380b63b07fd0af3d5a3f144cc86fb39fdd1/lib/vectortilefeature.js#L129
export function project(x: number, y: number, z: number, points) {
    const extent = 4096;
    const size = extent * Math.pow(2, z);
    const x0 = extent * x;
    const y0 = extent * y;
    if (points.length === 0) {
        throw new Error('No points')
    }
    const projected = new Array(points.length);
    for (let j = 0; j < points.length; j++) {
        const point = points[j];
        const y2 = 180 - (point.y + y0) * 360 / size;
        projected[j] = [
            (point.x + x0) * 360 / size - 180,
            360 / Math.PI * Math.atan(Math.exp(y2 * Math.PI / 180)) - 90
        ];
    }
    return projected
}
