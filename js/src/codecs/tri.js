import { EMPTY_U32, allocU32, checkScratchBytes } from "./binary.js";

let triSizes = new Int32Array(4096);
let triFrames = new Int32Array(2048);

function buildSubtreeSizes(symbols, sizes, triangles) {
  const symI32 = new Int32Array(
    symbols.buffer,
    symbols.byteOffset,
    triangles,
  );
  sizes[triangles] = 0;
  sizes[(triangles - 1) | 0] = 4; // (1 << 2) | 0
  let prevSz = 1;
  let bad = 0;
  for (let node = (triangles - 2) | 0; node >= 0; node = (node - 1) | 0) {
    const sym = symI32[node];
    bad |= sym >>> 2;
    const hasLeft = sym & 1;
    const hasRight = (sym >>> 1) & 1;
    // `prevSz` holds `sizes[node + 1] >>> 2` in a CPU register (zero store-to-load memory stall!)
    const leftSize = prevSz & -hasLeft;
    const rightRoot = (node + 1 + leftSize) | 0;
    const rightEntry = sizes[rightRoot] >>> 2;
    bad |= hasRight & ((rightEntry - 1) >>> 31);
    prevSz = (1 + leftSize + (rightEntry & -hasRight)) | 0;
    sizes[node] = (prevSz << 2) | sym;
  }
  return bad;
}

function unfoldAllTrees(sizes, cyclesI32, frames, outI32, triangles, cyclesLen) {
  let outPos = 0;
  let root = 0;
  let cyclePos = 0;

  while (root < triangles) {
    let cur = sizes[root];
    const treeSize = cur >>> 2;
    const nextCyclePos = (cyclePos + 2 + treeSize) | 0;
    if (nextCyclePos > cyclesLen) return -1;

    frames[0] = cyclesI32[cyclePos];
    frames[1] = cyclesI32[(cyclePos + 1) | 0];
    frames[2] = 0;
    const apexBase = (cyclePos + 2) | 0;
    const treeEnd = (root + treeSize) | 0;
    let sp = 4;

    for (let node = root; node < treeEnd; node = (node + 1) | 0) {
      sp = (sp - 4) | 0;
      const from = frames[sp];
      const to = frames[(sp + 1) | 0];
      const firstApex = frames[(sp + 2) | 0];
      const next = sizes[(node + 1) | 0];
      const sym = cur & 3;
      cur = next;
      const hasLeft = sym & 1;
      const hasRight = sym >>> 1;
      const leftSize = (next >>> 2) & -hasLeft;
      const apex = cyclesI32[(apexBase + firstApex + leftSize) | 0];

      outI32[outPos] = from;
      outI32[(outPos + 1) | 0] = to;
      outI32[(outPos + 2) | 0] = apex;
      outPos = (outPos + 3) | 0;

      frames[sp] = from;
      frames[(sp + 1) | 0] = apex;
      frames[(sp + 2) | 0] = (firstApex + leftSize + 1) | 0;
      sp = (sp + (hasRight << 2)) | 0;

      frames[sp] = apex;
      frames[(sp + 1) | 0] = to;
      frames[(sp + 2) | 0] = firstApex;
      sp = (sp + (hasLeft << 2)) | 0;
    }

    root = treeEnd;
    cyclePos = nextCyclePos;
  }

  return cyclePos;
}

export function decodeTrees(symbols, cycles) {
  const triangles = symbols.length | 0;
  if (triangles === 0) {
    if (cycles.length !== 0) throw new Error("cycles");
    return EMPTY_U32;
  }
  if (symbols[(triangles - 1) | 0] !== 0) {
    throw new Error("tree symbols");
  }

  const frameWords = (triangles + 2) << 2;
  checkScratchBytes((triangles + 1 + frameWords) * 4);
  if (triSizes.length <= triangles) {
    triSizes = new Int32Array(triangles + 1);
  }
  const sizes = triSizes;
  if (buildSubtreeSizes(symbols, sizes, triangles) !== 0) {
    throw new Error("tree symbols");
  }

  if (triFrames.length < frameWords) {
    triFrames = new Int32Array(frameWords);
  }

  const out = allocU32(triangles * 3);
  const outI32 = new Int32Array(out.buffer);
  const cyclesI32 = new Int32Array(
    cycles.buffer,
    cycles.byteOffset,
    cycles.length,
  );
  const cyclesLen = cyclesI32.length | 0;

  const consumed = unfoldAllTrees(
    sizes,
    cyclesI32,
    triFrames,
    outI32,
    triangles,
    cyclesLen,
  );
  if (consumed !== cyclesLen) {
    throw new Error(consumed < 0 ? "cycles eof" : "cycles trailing");
  }

  if (triSizes.length > 131073) {
    triSizes = new Int32Array(4096);
    triFrames = new Int32Array(2048);
  }
  return out;
}
