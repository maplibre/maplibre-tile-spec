import { describe, expect, it } from "vitest";

import { decodeFastPforInt32 } from "../decoding/fastPforDecoder";
import { BLOCK_SIZE } from "../decoding/fastPforShared";
import { createFastPforEncoderWorkspace, encodeFastPforInt32WithWorkspace } from "./fastPforEncoder";

const GROWTH_MULTIPLIER = 3;
const BASE_ALTERNATING_MASK = 1;
const EXCEPTION_POS_A = 10;
const EXCEPTION_POS_B = 100;
const EXCEPTION_OUTLIER_VALUE = 7;
const UNDERSIZED_PREALLOCATED_STREAM = 1;

describe("FastPFOR encoder", () => {
    it("grows byteContainer when workspace capacity is too small", () => {
        const values = new Int32Array(BLOCK_SIZE);
        for (let i = 0; i < values.length; i++) values[i] = i * GROWTH_MULTIPLIER;

        const workspace = createFastPforEncoderWorkspace();
        workspace.byteContainer = new Uint8Array(0);

        const encoded = encodeFastPforInt32WithWorkspace(values, workspace);
        const decoded = decodeFastPforInt32(encoded, values.length);

        expect(decoded).toEqual(values);
        expect(workspace.byteContainer.length).toBeGreaterThan(0);
    });

    it("grows exception buffer when preallocated exception stream is too small", () => {
        const values = new Int32Array(BLOCK_SIZE);
        for (let i = 0; i < values.length; i++) values[i] = i & BASE_ALTERNATING_MASK;
        values[EXCEPTION_POS_A] = EXCEPTION_OUTLIER_VALUE;
        values[EXCEPTION_POS_B] = EXCEPTION_OUTLIER_VALUE;

        const workspace = createFastPforEncoderWorkspace();
        workspace.dataToBePacked[2] = new Int32Array(UNDERSIZED_PREALLOCATED_STREAM);

        const encoded = encodeFastPforInt32WithWorkspace(values, workspace);
        const decoded = decodeFastPforInt32(encoded, values.length);

        expect(decoded).toEqual(values);
        expect(workspace.dataToBePacked[2]).toBeDefined();
        expect(workspace.dataToBePacked[2]!.length).toBeGreaterThan(1);
    });

    it("rounds grown exception buffers to a multiple of 32", () => {
        const values = new Int32Array(BLOCK_SIZE);
        for (let i = 0; i < values.length; i++) values[i] = i & BASE_ALTERNATING_MASK;
        values[EXCEPTION_POS_A] = EXCEPTION_OUTLIER_VALUE;
        values[EXCEPTION_POS_B] = EXCEPTION_OUTLIER_VALUE;

        const workspace = createFastPforEncoderWorkspace();
        workspace.dataToBePacked[2] = new Int32Array(UNDERSIZED_PREALLOCATED_STREAM);

        encodeFastPforInt32WithWorkspace(values, workspace);

        expect(workspace.dataToBePacked[2]).toBeDefined();
        expect(workspace.dataToBePacked[2]!.length % 32).toBe(0);
    });
});
