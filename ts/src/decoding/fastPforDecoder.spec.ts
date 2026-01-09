import { describe, expect, it } from "vitest";

import { decodeFastPforInt32 } from "./fastPforDecoder";
import { BLOCK_SIZE } from "./fastPforShared";

describe("FastPFOR decoder", () => {
    it.todo("Add encoder -> decoder round-trip tests in PR7");

    it("throws on invalid alignedLength (negative)", () => {
        expect(() => decodeFastPforInt32(new Int32Array([-1]), 0)).toThrow();
    });

    it("throws on invalid alignedLength (not multiple of 256)", () => {
        expect(() => decodeFastPforInt32(new Int32Array([1]), 0)).toThrow();
    });

    it("throws when alignedLength exceeds output length", () => {
        expect(() => decodeFastPforInt32(new Int32Array([BLOCK_SIZE]), 10)).toThrow();
    });
});
