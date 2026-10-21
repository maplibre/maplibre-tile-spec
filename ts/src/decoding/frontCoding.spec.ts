import { describe, expect, it } from "vitest";
import { decodeFrontCoded } from "./frontCoding";

const text = (bytes: Uint8Array, offsets: Uint32Array, i: number) =>
    Buffer.from(bytes.subarray(offsets[i], offsets[i + 1])).toString("utf8");

describe("decodeFrontCoded", () => {
    it("rebuilds the encodings doc example", () => {
        const lengths = new Uint32Array([0, 11, 2, 11, 6, 10]);
        const suffixes = new Uint8Array(Buffer.from("Main Street Northple Avenue"));
        const { offsets, bytes } = decodeFrontCoded(lengths, suffixes);
        expect(Array.from(offsets)).toEqual([0, 11, 28, 40]);
        expect([0, 1, 2].map((i) => text(bytes, offsets, i))).toEqual([
            "Main Street",
            "Main Street North",
            "Maple Avenue",
        ]);
    });

    it("handles an empty dictionary, empty entries, repeats and prefixes of the next entry", () => {
        expect(Array.from(decodeFrontCoded(new Uint32Array(0), new Uint8Array(0)).offsets)).toEqual([0]);
        const lengths = new Uint32Array([0, 0, 0, 3, 8, 0, 1, 3, 5, 0]);
        const suffixes = new Uint8Array(Buffer.from("asan jose"));
        const { offsets, bytes } = decodeFrontCoded(lengths, suffixes);
        expect([0, 1, 2, 3, 4].map((i) => text(bytes, offsets, i))).toEqual(["", "a", "san", "san jose", "san jose"]);
    });

    it("splits a multi-byte character across prefix and suffix", () => {
        // "aé", "aê": the second shares the first two bytes of three
        const { offsets, bytes } = decodeFrontCoded(
            new Uint32Array([0, 2, 3, 1]),
            new Uint8Array([0x61, 0xc3, 0xa9, 0xaa]),
        );
        expect([0, 1].map((i) => text(bytes, offsets, i))).toEqual(["aé", "aê"]);
    });

    it("rejects an odd lengths stream, a prefix longer than its predecessor, and leftover bytes", () => {
        expect(() => decodeFrontCoded(new Uint32Array([0, 1, 2]), new Uint8Array(3))).toThrow(/two per entry/);
        expect(() => decodeFrontCoded(new Uint32Array([0, 9, 2, 1]), new Uint8Array(3))).toThrow(/shares 9/);
        expect(() => decodeFrontCoded(new Uint32Array([0, 2]), new Uint8Array(3))).toThrow(/blob holds 3/);
        expect(() => decodeFrontCoded(new Uint32Array([1, 2]), new Uint8Array(2))).toThrow(/shares 1/);
    });
});
