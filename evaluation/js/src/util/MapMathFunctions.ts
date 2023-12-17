// eslint-disable-next-line @typescript-eslint/no-namespace
/**
 * Represents a point in any coordinate system
 */
export interface IPoint {
    /** The x coordinate */
    x: number;
    /** The y coordinate */
    y: number;
    /** The z coordinate */
    z?: number;
}
const tileLength = 256;

/**
 * convert lat/lon to a position on screen
 * @param zoom - tilesize is 256x256, so this zoom number is 1 bigger
 */
export function latLongToPixelXY(longitude: number, latitude: number, zoom: number) {
    zoom = zoom || 1;
    zoom = Math.round(zoom);
    const x = (longitude + 180) / 360;
    const sinLatitude = Math.sin(degreesToRadians(latitude));

    const y = 0.5 - Math.log((1 + sinLatitude) / (1 - sinLatitude)) / (4 * Math.PI);
    const size = getMapSize(zoom);
    const resultX = Math.floor(_clamp(x * size + 0.5, 0, size - 1));
    const resultY = Math.floor(_clamp(y * size + 0.5, 0, size - 1));
    return { x: resultX, y: resultY };
}

/**
 * Converts pixel XY coordinates into tile XY coordinates of the tile containing the
 * specified pixel
 * @param pixelXY - Pixel coordinates
 * @returns The tile coordinates of the tile containing the specified pixel
 */
export function pixelXYToTileXY(pixelXY: IPoint) {
    const tileX = Math.floor(pixelXY.x / tileLength);
    const tileY = Math.floor(pixelXY.y / tileLength);
    return { x: tileX, y: tileY };
}

export function pixelXYToTileOffset(pixelXY: IPoint) {
    /** In the world view pixel.x could be negative number and offset value suppose to be positive */
    const offsetX = pixelXY.x % tileLength;
    const offX = Math.floor(offsetX < 0 ? offsetX + tileLength : offsetX);
    const offY = Math.floor(pixelXY.y % tileLength);
    return { x: offX, y: offY };
}

export function getMapSize(levelOfDetail: number) {
    return tileLength << levelOfDetail;
}

/** The number of radians per degree */
const radiansPerDegree = Math.PI / 180;
export function degreesToRadians(degrees: number) {
    return degrees * radiansPerDegree;
}

export function _clamp(value: number, min: number, max: number) {
    return Math.min(Math.max(value, min), max);
}

export function getQuadKey(x: number, y: number, z: number) {
    const quadKey = [];
    for (let i = z; i > 0; i--) {
        let digit = 0;
        const mask = 1 << (i - 1);
        if ((x & mask) != 0) {
            digit++;
        }
        if ((y & mask) != 0) {
            digit++;
            digit++;
        }
        quadKey.push(digit);
    }
    return quadKey.join("");
}
