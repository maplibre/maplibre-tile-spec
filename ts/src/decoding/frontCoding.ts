/**
 * Front coding (`docs/encodings.md#front-coding`): a sorted dictionary stored as, per entry, the
 * length of the prefix it shares with the entry before it and only its suffix bytes.
 *
 * The lengths stream holds `2N` values, `N` prefix lengths then `N` suffix lengths, and the blob
 * holds the suffixes back to back. The reference is `codecs/front_coding.rs` in mlt-core.
 */

import { chargeBytes } from "./v2/binary";

export interface FrontDecoded {
    /** Cumulative byte offsets of the entries into `bytes`, `N + 1` of them. */
    offsets: Uint32Array;
    bytes: Uint8Array;
}

export function decodeFrontCoded(lengths: Uint32Array, suffixes: Uint8Array): FrontDecoded {
    if (lengths.length % 2 !== 0) {
        throw new Error(`Front-coded lengths stream holds ${lengths.length} values, not two per entry`);
    }
    const count = lengths.length / 2;
    let total = 0;
    for (let i = 0; i < count; i++) {
        total += lengths[count + i];
    }
    if (total !== suffixes.length) {
        throw new Error(`Front-coded suffixes take ${total} bytes but the blob holds ${suffixes.length}`);
    }
    let size = 0;
    let previousLength = 0;
    for (let i = 0; i < count; i++) {
        const shared = lengths[i];
        if (shared > previousLength) {
            throw new Error(`Front-coded entry ${i} shares ${shared} bytes with a predecessor of ${previousLength}`);
        }
        previousLength = shared + lengths[count + i];
        size += previousLength;
    }
    chargeBytes(size + (count + 1) * 4);
    const bytes = new Uint8Array(size);
    const offsets = new Uint32Array(count + 1);
    let at = 0;
    let suffixAt = 0;
    let previousStart = 0;
    for (let i = 0; i < count; i++) {
        const shared = lengths[i];
        const suffixLength = lengths[count + i];
        bytes.copyWithin(at, previousStart, previousStart + shared);
        bytes.set(suffixes.subarray(suffixAt, suffixAt + suffixLength), at + shared);
        suffixAt += suffixLength;
        previousStart = at;
        at += shared + suffixLength;
        offsets[i + 1] = at;
    }
    return { offsets, bytes };
}
