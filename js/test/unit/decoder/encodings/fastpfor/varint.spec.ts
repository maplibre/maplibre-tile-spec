import fs from "fs";
import {VarInt} from "../../../../../src/encodings/fastpfor/varint";
import {DecodingUtils} from "../../../../../src/decoder/DecodingUtils";


describe("VarInt", () => {
    it("should decode continuously ascending values", async () => {
        const compressed = new Uint32Array(fs.readFileSync("test/data/250k_ascending_varint.bin").buffer);
        const varint = VarInt.default();

        const uncompressed = varint.uncompress({
            input: compressed,
        });

        const zigzag = Array.from(uncompressed);
        DecodingUtils.decodeZigZagArray(zigzag);


        const expected = new Uint32Array(250_000);
        for (let i = 0; i < 250_000; i++) {
            expected[i] = i;
        }

        expect(expected).toEqual(Uint32Array.from(uncompressed));
    });

    it("should decode stepped ascending values", async () => {
        const compressed = new Uint32Array(fs.readFileSync("test/data/250k_step_varint.bin").buffer);
        const varint = VarInt.default();

        const uncompressed = varint.uncompress({
            input: compressed,
        });

        const zigzag = Array.from(uncompressed);
        DecodingUtils.decodeZigZagArray(zigzag);

        const expected = new Uint32Array(250_000);
        for (let i = 0; i < 250_000; i++) {
            if (i % 65536 == 0)
                expected[i] = 1000000000;
            else
                expected[i] = i % 4096;
        }

        expect(expected).toEqual(Uint32Array.from(uncompressed));
    });
});
