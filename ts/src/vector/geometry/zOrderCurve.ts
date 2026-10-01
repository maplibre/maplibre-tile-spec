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
