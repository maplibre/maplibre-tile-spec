import { describe, expect, it } from "vitest";
import { decodeStreamHeaders } from "../decoding/decodingTestUtils";
import { LogicalLevelTechnique } from "../metadata/tile/logicalLevelTechnique";
import { PhysicalLevelTechnique } from "../metadata/tile/physicalLevelTechnique";
import { PhysicalStreamType } from "../metadata/tile/physicalStreamType";
import { encodeBooleanNullableColumn, encodeDoubleNullableColumn, encodeFloatNullableColumn } from "./propertyEncoder";

describe("propertyEncoder", () => {
    it("writes a nullable boolean column as a Present stream and a data stream, both byte-RLE", () => {
        const headers = decodeStreamHeaders(encodeBooleanNullableColumn([true, null, false]));

        const byteRle = {
            logicalLevelTechnique1: LogicalLevelTechnique.RLE,
            physicalLevelTechnique: PhysicalLevelTechnique.NONE,
        };
        expect(headers).toMatchObject([
            { ...byteRle, physicalStreamType: PhysicalStreamType.PRESENT, numValues: 3 },
            { ...byteRle, physicalStreamType: PhysicalStreamType.DATA, numValues: 2 },
        ]);
    });

    it("writes the data of float and double columns as fixed-width words", () => {
        const floats = decodeStreamHeaders(encodeFloatNullableColumn([1.5, null, -2.25]));
        const doubles = decodeStreamHeaders(encodeDoubleNullableColumn([1.5, null, -2.25]));

        const present = { physicalStreamType: PhysicalStreamType.PRESENT };
        const fixedWidth = {
            physicalStreamType: PhysicalStreamType.DATA,
            logicalLevelTechnique1: LogicalLevelTechnique.NONE,
            physicalLevelTechnique: PhysicalLevelTechnique.NONE,
        };
        expect(floats).toMatchObject([present, { ...fixedWidth, byteLength: 8 }]);
        expect(doubles).toMatchObject([present, { ...fixedWidth, byteLength: 16 }]);
    });
});
