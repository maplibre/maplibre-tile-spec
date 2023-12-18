/* WARNING: 
   This file is provided for academic purpose only.
   Any other usage is prohibited.
   There is no guarantee that any of the APIs in this file will remain functioning.
 */

export type TileCoordinate = {
    x: number;
    y: number;
    z: number;
};

/**
 * @param targetDataZoom the data zoom being used to compute URL.
 * @param width Width of the map on screen. Notice on high DPI machines the map canvas is actullay much bigger
 * @param height Height of the map on screen.
 */
export function getViewTileSets(
    lat: number,
    lon: number,
    targetDataZoom: number,
    width: number,
    height: number,
): TileCoordinate[] {
    const centerPixel = latLongToPixelXY(lon, lat, targetDataZoom);

    // other calculations are based on tile size 256 while mvt tilesize is 512 so scale is 2
    const scale = 2;

    const widthAtTargetZoom = width / scale;
    const heightAtTargetZoom = height / scale;
    const halfViewportWidth = widthAtTargetZoom / 2;
    const halfViewportHeight = heightAtTargetZoom / 2;

    const topLeftPixel = {
        x: centerPixel.x - halfViewportWidth,
        y: centerPixel.y - halfViewportHeight,
    };
    const bottomRightPixel = {
        x: centerPixel.x + halfViewportWidth - 1,
        y: centerPixel.y + halfViewportHeight - 1,
    };
    const topLeftTile = pixelXYToTileXY(topLeftPixel);
    const bottomRightTile = pixelXYToTileXY(bottomRightPixel);

    const results: TileCoordinate[] = [];

    for (let x = topLeftTile.x; x <= bottomRightTile.x; x++) {
        for (let y = topLeftTile.y; y <= bottomRightTile.y; y++) {
            const posCoords = { x, y, z: targetDataZoom };
            const coords = normalizeTileCoords(posCoords);
            results.push(coords);
        }
    }

    return results;
}

function normalizeTileCoords(coords: TileCoordinate): TileCoordinate {
    const maxTiles = 1 << coords.z;
    const { x, y, z } = coords;

    return {
        x: x < 0 ? (x % maxTiles) + maxTiles : x % maxTiles,
        y: y < 0 ? (y % maxTiles) + maxTiles : y % maxTiles,
        z: z,
    };
}

/**
 * Represents a point in any coordinate system
 */
interface IPoint {
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
function latLongToPixelXY(longitude: number, latitude: number, zoom: number) {
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
function pixelXYToTileXY(pixelXY: IPoint) {
    const tileX = Math.floor(pixelXY.x / tileLength);
    const tileY = Math.floor(pixelXY.y / tileLength);
    return { x: tileX, y: tileY };
}

function getMapSize(levelOfDetail: number) {
    return tileLength << levelOfDetail;
}

/** The number of radians per degree */
const radiansPerDegree = Math.PI / 180;
function degreesToRadians(degrees: number) {
    return degrees * radiansPerDegree;
}

function _clamp(value: number, min: number, max: number) {
    return Math.min(Math.max(value, min), max);
}
