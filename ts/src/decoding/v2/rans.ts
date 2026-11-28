import { EMPTY_I32, allocI32, checkScratchBytes, cursor, readBytes, readVarintU32, scratchU32 } from "./binary";

const STATE_LOW = 65536;
const SYMBOLS = 257;
const CONTEXTS = 14;

const lowMask = (b: number): number => (b === 0 ? 0 : b === 32 ? 0xffffffff : (1 << b) - 1);

interface SymInfo {
    rawBits: Uint8Array;
    negFlag: Uint8Array;
    dyCtxBase: Uint8Array;
    head: Uint8Array;
    valid: Uint8Array;
}

let symInfo: SymInfo[] | null = null;

function getSymInfo(): SymInfo[] {
    if (symInfo !== null) return symInfo;
    symInfo = [0, 1, 2].map((depth) => {
        const rawBits = new Uint8Array(SYMBOLS);
        const negFlag = new Uint8Array(SYMBOLS);
        const dyCtxBase = new Uint8Array(SYMBOLS);
        const head = new Uint8Array(SYMBOLS);
        const valid = new Uint8Array(SYMBOLS);
        valid[0] = 1;
        dyCtxBase[0] = 2;
        for (let s = 1; s < SYMBOLS; s++) {
            const code = (s + 1) >>> 1;
            const len = code >>> depth;
            if (len < 1 || len > 32) continue;
            const kept = Math.min(len - 1, depth);
            const mantissa = code & lowMask(depth);
            if ((mantissa & lowMask(depth - kept)) !== 0) continue;
            const isNeg = (s & 1) === 0;
            if (len === 32 && (!isNeg || mantissa !== 0)) continue;
            valid[s] = 1;
            rawBits[s] = len - 1 - kept;
            negFlag[s] = isNeg ? 1 : 0;
            dyCtxBase[s] = 2 + 2 * Math.min(len, 5);
            head[s] = (1 << kept) | (mantissa >>> (depth - kept));
        }
        return { rawBits, negFlag, dyCtxBase, head, valid };
    });
    return symInfo;
}

export function decodeRansVertices(
    data: Uint8Array,
    runOffsets: Uint32Array | undefined,
    numWords: number,
): Int32Array {
    if ((numWords & 1) !== 0) throw new Error("rans odd");
    const vertexCount = numWords >>> 1;
    checkScratchBytes(vertexCount);
    const first = new Uint8Array(vertexCount);
    if (runOffsets === undefined) {
        first.fill(1);
    } else {
        if (
            (runOffsets.length > 0 && runOffsets[0] !== 0) ||
            (runOffsets[runOffsets.length - 1] ?? 0) !== vertexCount
        ) {
            throw new Error("rans runs");
        }
        for (let i = 0; i < runOffsets.length; i++) {
            const o = runOffsets[i];
            if (o < vertexCount) first[o] = 1;
            else if (o !== vertexCount) throw new Error("rans runs");
        }
    }

    if (data.length === 0 || data[0] > 2) throw new Error("rans hdr");
    const info = getSymInfo()[data[0]];
    const tBytes = data.subarray(1);
    let tBit = 0;

    const getBits = (bits: number): number => {
        if (bits === 0) return 0;
        const bi = tBit >>> 3;
        const w = (tBytes[bi] ?? 0) | ((tBytes[bi + 1] ?? 0) << 8) | ((tBytes[bi + 2] ?? 0) << 16);
        const res = (w >>> (tBit & 7)) & lowMask(bits);
        tBit += bits;
        return res;
    };
    const getGamma = (): number => {
        for (let z = 0; z <= 12; z++) {
            if (getBits(1) === 1) return (1 << z) | getBits(z);
        }
        throw new Error("rans freq");
    };

    const tablePrec = new Int8Array(CONTEXTS).fill(-1);
    const tableFreqs: Array<Uint32Array | undefined> = new Array(CONTEXTS);
    let usedSlots = 1;

    for (let c = 0; c < CONTEXTS; c++) {
        const present = getBits(1);
        if (tBit > tBytes.length * 8) throw new Error("rans trunc");
        if (present === 0) continue;
        const precision = getBits(4);
        const max = getBits(9);
        if (precision > 11 || max >= SYMBOLS) throw new Error("rans table");
        const freq = scratchU32(max + 1);
        let sum = 0;
        let bad = 0;
        if (precision === 0) {
            freq[max] = 1;
            sum = 1;
            if (!info.valid[max]) bad = 1;
        } else {
            for (let s = 0; s <= max; s++) {
                const f = getGamma() - 1;
                freq[s] = f;
                sum += f;
                if (f > 0 && !info.valid[s]) bad = 1;
            }
        }
        if (tBit > tBytes.length * 8 || sum !== 1 << precision || freq[max] === 0 || bad) {
            throw new Error("rans freq");
        }
        tablePrec[c] = precision;
        tableFreqs[c] = freq;
        usedSlots += 1 << precision;
    }

    let slabSize = 4096;
    while (slabSize < usedSlots) slabSize <<= 1;
    const slabOff = new Uint16Array(slabSize);
    const slabFreq = new Uint16Array(slabSize);
    const slabSym = new Uint16Array(slabSize);
    const slabInvalid = new Uint8Array(slabSize);
    const ctxStart = new Uint32Array(CONTEXTS);
    const ctxPrecision = new Uint8Array(CONTEXTS);
    slabFreq[0] = 1;
    slabInvalid[0] = 1;

    let at = 1;
    for (let c = 0; c < CONTEXTS; c++) {
        const prec = tablePrec[c];
        const freq = tableFreqs[c];
        if (prec < 0 || !freq) continue;
        ctxStart[c] = at;
        ctxPrecision[c] = prec;
        for (let s = 0; s < freq.length; s++) {
            const f = freq[s];
            for (let k = 0; k < f; k++) {
                slabOff[at + k] = k;
                slabFreq[at + k] = f;
                slabSym[at + k] = s;
            }
            at += f;
        }
    }

    const r = cursor(data, 1 + ((tBit + 7) >>> 3), data.length);
    const laneLens = [readVarintU32(r), readVarintU32(r), readVarintU32(r), readVarintU32(r)];
    const laneBytes: Uint8Array[] = [];
    const lanePos = new Uint32Array(4);
    const laneState = new Uint32Array(4);
    for (let l = 0; l < 4; l++) {
        const len = laneLens[l];
        if (len < 4 || (len & 1) !== 0 || r.e - r.p < len) throw new Error("rans lane");
        const bytes = readBytes(r, len);
        laneBytes.push(bytes);
        lanePos[l] = 4;
        laneState[l] = (bytes[0] | (bytes[1] << 8) | (bytes[2] << 16) | (bytes[3] << 24)) >>> 0;
    }

    const raw = readBytes(r, r.e - r.p);
    let rawBit = 0;
    const slabMask = slabSize - 1;

    const stepLane = (l: number, c: number): number => {
        const prec = ctxPrecision[c];
        let st = laneState[l];
        const slot = (ctxStart[c] + (st & ((1 << prec) - 1))) & slabMask;
        st = (Math.imul(slabFreq[slot], st >>> prec) + slabOff[slot]) >>> 0;
        if (st < STATE_LOW) {
            const bytes = laneBytes[l];
            const p = lanePos[l];
            const word = p + 2 <= bytes.length ? bytes[p] | (bytes[p + 1] << 8) : 0;
            st = ((st << 16) | word) >>> 0;
            lanePos[l] = p + 2;
        }
        laneState[l] = st;
        return slot;
    };

    const readDelta = (sym: number): number => {
        const rb = info.rawBits[sym];
        let low = 0;
        if (rb > 0) {
            const bi = rawBit >>> 3;
            const sh = rawBit & 7;
            const lo32 =
                ((raw[bi] ?? 0) |
                    ((raw[bi + 1] ?? 0) << 8) |
                    ((raw[bi + 2] ?? 0) << 16) |
                    ((raw[bi + 3] ?? 0) << 24)) >>>
                0;
            low =
                sh === 0
                    ? (lo32 & lowMask(rb)) >>> 0
                    : (((lo32 >>> sh) | ((raw[bi + 4] ?? 0) << (32 - sh))) & lowMask(rb)) >>> 0;
            rawBit += rb;
        }
        const mag = (info.head[sym] << rb) | low | 0;
        const sign = -info.negFlag[sym];
        return ((mag ^ sign) - sign) | 0;
    };

    const out = numWords === 0 ? EMPTY_I32 : allocI32(numWords);
    let x = 0;
    let y = 0;
    let bad = 0;
    for (let v = 0; v < vertexCount; v += 2) {
        const f0 = first[v];
        const ex0 = stepLane(0, f0);
        const hasPair = v + 1 < vertexCount;
        const f1 = hasPair ? first[v + 1] : 0;
        const ex1 = hasPair ? stepLane(2, f1) : 0;
        const ey0 = stepLane(1, info.dyCtxBase[slabSym[ex0]] + f0);
        const ey1 = hasPair ? stepLane(3, info.dyCtxBase[slabSym[ex1]] + f1) : 0;

        bad |= slabInvalid[ex0] | slabInvalid[ey0];
        x = (x + readDelta(slabSym[ex0])) | 0;
        y = (y + readDelta(slabSym[ey0])) | 0;
        out[v * 2] = x;
        out[v * 2 + 1] = y;
        if (hasPair) {
            bad |= slabInvalid[ex1] | slabInvalid[ey1];
            x = (x + readDelta(slabSym[ex1])) | 0;
            y = (y + readDelta(slabSym[ey1])) | 0;
            out[v * 2 + 2] = x;
            out[v * 2 + 3] = y;
        }
    }

    if (bad !== 0) throw new Error("rans missing table");
    for (let l = 0; l < 4; l++) {
        if (laneState[l] !== STATE_LOW || lanePos[l] !== laneBytes[l].length) throw new Error("rans state");
    }
    if ((rawBit + 7) >>> 3 !== raw.length) throw new Error("rans raw len");
    return out;
}
