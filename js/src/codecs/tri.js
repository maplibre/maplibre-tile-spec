import { EMPTY_U32 } from "./binary.js";

// Int32Array scratch buffers ensure V8 TurboFan keeps all arithmetic in pure 32-bit GP registers
// (Signed32) without widening u32 products/indices to float64.
let triSizes = new Int32Array(4096);
let frameFrom = new Int32Array(512);
let frameTo = new Int32Array(512);
let frameApex = new Int32Array(512);

export function decodeTrees(symbols, cycles) {
  const triangles = symbols.length | 0;
  if (triangles === 0) {
    if (cycles.length !== 0) throw new Error("cycles");
    return EMPTY_U32;
  }

  if (triSizes.length <= triangles) {
    triSizes = new Int32Array(triangles + 1);
  }
  const sizes = triSizes;
  sizes[triangles] = 0;

  let bad = 0;
  for (let node = (triangles - 1) | 0; node >= 0; node = (node - 1) | 0) {
    const sym = symbols[node] | 0;
    bad |= sym >>> 2;
    const hasLeft = sym & 1;
    const hasRight = (sym >>> 1) & 1;
    const leftSize = sizes[(node + 1) | 0] & -hasLeft;
    const rightRoot = (node + 1 + leftSize) | 0;
    const safeRight = rightRoot <= triangles ? rightRoot : triangles;
    const rightSize = sizes[safeRight] & -hasRight;
    bad |=
      (hasLeft & +(node + 1 === triangles)) |
      (hasRight & +(rightRoot >= triangles));
    sizes[node] = (1 + leftSize + rightSize) | 0;
  }
  if (bad !== 0) throw new Error("tree symbols");

  const reqCap = (triangles + 2) | 0;
  if (frameFrom.length < reqCap) {
    frameFrom = new Int32Array(reqCap);
    frameTo = new Int32Array(reqCap);
    frameApex = new Int32Array(reqCap);
  }
  const fFrom = frameFrom;
  const fTo = frameTo;
  const fApex = frameApex;

  const out = new Uint32Array(triangles * 3);
  const cyclesLen = cycles.length | 0;
  let outPos = 0;
  let root = 0;
  let cyclePos = 0;

  while (root < triangles) {
    const treeSize = sizes[root];
    const nextCyclePos = (cyclePos + 2 + treeSize) | 0;
    if (nextCyclePos > cyclesLen) throw new Error("cycles eof");

    fFrom[0] = cycles[cyclePos] | 0;
    fTo[0] = cycles[(cyclePos + 1) | 0] | 0;
    fApex[0] = 0;
    const apexBase = (cyclePos + 2) | 0;
    const treeEnd = (root + treeSize) | 0;
    let pending = 1;

    for (let node = root; node < treeEnd; node = (node + 1) | 0) {
      pending = (pending - 1) | 0;
      const from = fFrom[pending];
      const to = fTo[pending];
      const firstApex = fApex[pending];
      const sym = symbols[node] | 0;
      const hasLeft = sym & 1;
      const hasRight = sym >>> 1;
      const leftSize = sizes[(node + 1) | 0] & -hasLeft;
      const apex = cycles[(apexBase + firstApex + leftSize) | 0] | 0;

      out[outPos] = from;
      out[(outPos + 1) | 0] = to;
      out[(outPos + 2) | 0] = apex;
      outPos = (outPos + 3) | 0;

      fFrom[pending] = from;
      fTo[pending] = apex;
      fApex[pending] = (firstApex + leftSize + 1) | 0;
      pending = (pending + hasRight) | 0;

      fFrom[pending] = apex;
      fTo[pending] = to;
      fApex[pending] = firstApex;
      pending = (pending + hasLeft) | 0;
    }

    root = treeEnd;
    cyclePos = nextCyclePos;
  }

  if (cyclePos !== cyclesLen) throw new Error("cycles trailing");

  if (triSizes.length > 131073) {
    triSizes = new Int32Array(4096);
    frameFrom = new Int32Array(512);
    frameTo = new Int32Array(512);
    frameApex = new Int32Array(512);
  }
  return out;
}
