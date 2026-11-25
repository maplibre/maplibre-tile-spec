import { describe, expect, it } from "vitest";
import { encodeUtf8Strings } from "./encodingUtils";

describe("encodeUtf8Strings", () => {
    it("should give the bytes and byte lengths of TextEncoder for every UTF-8 sequence length", () => {
        const strings = [
            "",
            "ascii",
            "café",
            "日本語",
            "a\u{1f600}b",
            "lone high \ud83d",
            "\ude00 lone low",
            "",
            "ascii after the others",
        ];
        const expected = strings.map((string) => new TextEncoder().encode(string));

        const { bytes, lengths } = encodeUtf8Strings(strings);

        expect(Array.from(bytes)).toEqual(expected.flatMap((encoded) => Array.from(encoded)));
        expect(Array.from(lengths)).toEqual(expected.map((encoded) => encoded.length));
    });
});
