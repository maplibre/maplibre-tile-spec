import { describe, it, expect, beforeAll } from "vitest";
import decodeTile from "../../mltDecoder";
import * as fs from "node:fs";
import * as path from "node:path";

describe("embeddedTilesetMetadataDecoder", () => {
    let fixture: Uint8Array;

    beforeAll(() => {
        fixture = new Uint8Array(fs.readFileSync(path.resolve(__dirname, "../../test/fixtures/struct/4_8_5.mlt")));
    });

    it("should decode tile with nested structs", () => {
        expect(() => decodeTile(fixture)).not.toThrow();

        const tables = decodeTile(fixture);
        expect(tables.length).toBeGreaterThan(0);
        expect(tables[0].name).toBeDefined();
        expect(tables[0].extent).toBeGreaterThan(0);
    });
});
