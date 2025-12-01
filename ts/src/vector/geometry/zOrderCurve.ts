import SpaceFillingCurve from "./spaceFillingCurve";

export function decodeZOrderCurve(
    mortonCode: number,
    numBits: number,
    coordinateShift: number,
): { x: number; y: number } {
    const x = decodeMorton(mortonCode, numBits) - coordinateShift;
    const y = decodeMorton(mortonCode >> 1, numBits) - coordinateShift;
    return { x, y };
}

function decodeMorton(code: number, numBits: number): number {
    let coordinate = 0;
    for (let i = 0; i < numBits; i++) {
        coordinate |= (code & (1 << (2 * i))) >> i;
    }
    return coordinate;
}

export default class ZOrderCurve extends SpaceFillingCurve {
    encode(vertex: { x: number; y: number }): number {
        this.validateCoordinates(vertex);
        const shiftedX = vertex.x + this._coordinateShift;
        const shiftedY = vertex.y + this._coordinateShift;
        let mortonCode = 0;
        for (let i = 0; i < this._numBits; i++) {
            mortonCode |= ((shiftedX & (1 << i)) << i) | ((shiftedY & (1 << i)) << (i + 1));
        }
        return mortonCode;
    }

    decode(mortonCode: number): { x: number; y: number } {
        return decodeZOrderCurve(mortonCode, this._numBits, this._coordinateShift);
    }
}
