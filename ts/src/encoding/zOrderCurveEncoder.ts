export function encodeZOrderCurve(
    x: number,
    y: number,
    numBits: number,
    coordinateShift: number,
): number {
    const shiftedX = x + coordinateShift;
    const shiftedY = y + coordinateShift;
    let code = 0;
    for(let i = 0; i < numBits; i++) {
        code |= (shiftedX & (1 << i)) << i | (shiftedY & (1 << i)) << (i + 1);
    }
    return code
}
