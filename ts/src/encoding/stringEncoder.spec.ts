import { describe, expect, it } from "vitest";
import { decodeStreamHeaders } from "../decoding/decodingTestUtils";
import { LogicalLevelTechnique } from "../metadata/tile/logicalLevelTechnique";
import { PhysicalLevelTechnique } from "../metadata/tile/physicalLevelTechnique";
import { PhysicalStreamType } from "../metadata/tile/physicalStreamType";
import { encodeDictionaryStrings, encodePlainStrings } from "./stringEncoder";

describe("stringEncoder", () => {
    it("writes plain strings as a byte-RLE Present stream, their lengths, then their bytes", () => {
        const headers = decodeStreamHeaders(encodePlainStrings(["a", null]));

        expect(headers).toMatchObject([
            {
                physicalStreamType: PhysicalStreamType.PRESENT,
                logicalLevelTechnique1: LogicalLevelTechnique.RLE,
                physicalLevelTechnique: PhysicalLevelTechnique.NONE,
            },
            { physicalStreamType: PhysicalStreamType.LENGTH },
            { physicalStreamType: PhysicalStreamType.DATA },
        ]);
    });

    it("writes a dictionary as its Present stream, the lengths of its entries, the offsets into it, then its bytes", () => {
        const streams = decodeStreamHeaders(encodeDictionaryStrings(["a", null, "a"]));

        expect(streams.map((stream) => stream.physicalStreamType)).toEqual([
            PhysicalStreamType.PRESENT,
            PhysicalStreamType.LENGTH,
            PhysicalStreamType.OFFSET,
            PhysicalStreamType.DATA,
        ]);
        expect(streams[0]).toMatchObject({
            logicalLevelTechnique1: LogicalLevelTechnique.RLE,
            physicalLevelTechnique: PhysicalLevelTechnique.NONE,
        });
    });
});
