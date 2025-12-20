import { describe, expect, it } from "vitest";

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { decodeFastPfor } from "./integerDecodingUtils";
import IntWrapper from "./intWrapper";
import { int32sToBigEndianBytes, uncompressFastPforInt32 } from "../fastPforCodec";

function parseCppUint32Array(source: string, name: string): Int32Array {
    const match = new RegExp(`const\\s+std::uint32_t\\s+${name}\\s*\\[\\]\\s*=\\s*\\{([\\s\\S]*?)\\};`).exec(source);
    if (!match) {
        throw new Error(`Failed to locate C++ array ${name}`);
    }

    const tokens = match[1]
        .split(",")
        .map((t) => t.trim())
        .filter((t) => t.length > 0);

    const values = new Int32Array(tokens.length);
    for (let i = 0; i < tokens.length; i++) {
        let token = tokens[i];
        token = token.replace(/u$/i, "");
        token = token.replace(/^UINT32_C\\((.*)\\)$/, "$1");
        token = token.replace(/^INT32_C\\((.*)\\)$/, "$1");

        const parsed = Number(token);
        if (!Number.isFinite(parsed)) {
            throw new Error(`Failed to parse token '${tokens[i]}' in ${name}`);
        }
        values[i] = parsed | 0;
    }

    return values;
}

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const CPP_FASTPFOR_TEST = path.resolve(__dirname, "../../../cpp/test/test_fastpfor.cpp");

describe("fastpfor decoder compatibility (JavaFastPFOR reference vectors)", () => {
    it("decodes cpp/test/test_fastpfor.cpp compressed1 -> uncompressed1", () => {
        const cpp = fs.readFileSync(CPP_FASTPFOR_TEST, "utf8");
        const encoded = parseCppUint32Array(cpp, "compressed1");
        const expected = parseCppUint32Array(cpp, "uncompressed1");

        const decoded = uncompressFastPforInt32(encoded, expected.length);
        expect(Array.from(decoded)).toEqual(Array.from(expected));

        const bytes = int32sToBigEndianBytes(encoded);
        const offset = new IntWrapper(0);
        const decodedFromBytes = decodeFastPfor(bytes, expected.length, bytes.length, offset);
        expect(Array.from(decodedFromBytes)).toEqual(Array.from(expected));
        expect(offset.get()).toBe(bytes.length);
    });

    it("decodes cpp/test/test_fastpfor.cpp compressed3 -> uncompressed3", () => {
        const cpp = fs.readFileSync(CPP_FASTPFOR_TEST, "utf8");
        const encoded = parseCppUint32Array(cpp, "compressed3");
        const expected = parseCppUint32Array(cpp, "uncompressed3");

        const decoded = uncompressFastPforInt32(encoded, expected.length);
        expect(Array.from(decoded)).toEqual(Array.from(expected));
    });
});
