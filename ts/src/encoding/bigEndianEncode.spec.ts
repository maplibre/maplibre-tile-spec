import { describe, expect, it } from "vitest";
import { encodeBigEndianInt32s } from "./bigEndianEncode";

describe("encodeBigEndianInt32s", () => {
    it("converts Int32Array to Big-Endian Uint8Array", () => {
        const input = new Int32Array([0x12345678, -1, 0, 1]);
        const output = encodeBigEndianInt32s(input);

        expect(output).toBeInstanceOf(Uint8Array);
        expect(output.length).toBe(16);

        expect(output[0]).toBe(0x12);
        expect(output[1]).toBe(0x34);
        expect(output[2]).toBe(0x56);
        expect(output[3]).toBe(0x78);

        expect(output[4]).toBe(0xff);
        expect(output[7]).toBe(0xff);
    });

    it("handles empty array", () => {
        const input = new Int32Array([]);
        const output = encodeBigEndianInt32s(input);
        expect(output.length).toBe(0);
    });

    it("handles single value", () => {
        const input = new Int32Array([0x01020304]);
        const output = encodeBigEndianInt32s(input);
        expect(output).toEqual(new Uint8Array([0x01, 0x02, 0x03, 0x04]));
    });
});
