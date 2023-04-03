import {Point} from "../util/geometry";


export function zOrder(point: Point): number {
    let x = Math.round(1024 * (fromZigZag(point.x) + 1024) / 6144);
    let y = Math.round(1024 * (fromZigZag(point.y) + 1024) / 6144);

    x = (x | (x << 8)) & 0x00FF00FF;
    x = (x | (x << 4)) & 0x0F0F0F0F;
    x = (x | (x << 2)) & 0x33333333;
    x = (x | (x << 1)) & 0x55555555;

    y = (y | (y << 8)) & 0x00FF00FF;
    y = (y | (y << 4)) & 0x0F0F0F0F;
    y = (y | (y << 2)) & 0x33333333;
    y = (y | (y << 1)) & 0x55555555;

    return x | (y << 1);
}

function fromZigZag(n) {
    return n % 2 === 0 ? n / 2 : -(n + 1) / 2;
}
