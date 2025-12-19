import { describe, expect, it, vi } from "vitest";

vi.mock("./geometryDecoder", () => ({
    decodeGeometryColumn: vi.fn(),
}));

import { decodeGeometryColumn } from "./geometryDecoder";
import { DeferredGeometryColumn } from "./deferredGeometryColumn";
import type { GeometryVector } from "../vector/geometry/geometryVector";

describe("DeferredGeometryColumn", () => {
    it("decodes only once and caches the result", () => {
        const fakeVector = {} as GeometryVector;
        const decodeSpy = vi.mocked(decodeGeometryColumn);
        decodeSpy.mockReturnValue(fakeVector);

        const deferred = new DeferredGeometryColumn({
            tile: new Uint8Array(0),
            numStreams: 1,
            numFeatures: 1,
            startOffset: 0,
        });

        const first = deferred.get();
        const second = deferred.get();

        expect(first).toBe(fakeVector);
        expect(second).toBe(fakeVector);
        expect(decodeSpy).toHaveBeenCalledTimes(1);
    });
});
