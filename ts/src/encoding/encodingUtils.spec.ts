import { describe, expect, it } from "vitest";
import {
    createStringLengths,
    encodeBooleanRle,
    encodeByteRle,
    encodeDoubleLE,
    encodeFloatsLE,
    encodeStrings,
    encodeUint32sLE,
    encodeUint64sLE,
} from "./encodingUtils";
import { packNullable } from "./packNullableUtils";
import {
    decodeBooleanRle,
    decodeByteRle,
    decodeDoublesLE,
    decodeFloatsLE,
    decodeString,
    decodeUint32sLE,
    decodeUint64sLE,
} from "../decoding/decodingUtils";
import { unpackNullable } from "../decoding/unpackNullableUtils";
import BitVector from "../vector/flat/bitVector";
import IntWrapper from "../decoding/intWrapper";

/**
 * A byte RLE header either opens a run of `header + 3` copies of the byte that follows it, or
 * introduces `256 - header` literal bytes, so one header carries at most 130 repeats or 128
 * literals. `docs/encodings.md` defers to the ORC specification for this, and
 * `rust/mlt-core/src/codecs/rle.rs` encodes to the same two limits.
 */
const MAX_RUN = 130;
const MAX_LITERALS = 128;

/** Fixed seeds keep the generated cases identical on every machine and every run. */
function seededRandom(seed: number): () => number {
    let state = seed >>> 0;
    return () => {
        state = (state + 0x6d2b79f5) >>> 0;
        let word = state;
        word = Math.imul(word ^ (word >>> 15), word | 1);
        word ^= word + Math.imul(word ^ (word >>> 7), word | 61);
        return ((word ^ (word >>> 14)) >>> 0) / 4294967296;
    };
}

/**
 * Replays an encoded stream using only the header rules, so the encoder is measured against the
 * format instead of against our own decoder. Returns "ok" or the first difference it finds.
 */
function replayByteRle(expected: Uint8Array, encoded: Uint8Array): string {
    const replayed: number[] = [];
    let pos = 0;

    while (pos < encoded.length) {
        const header = encoded[pos++];
        if (header < 0x80) {
            if (pos >= encoded.length) {
                return "a run header ended the stream without its value byte";
            }
            const value = encoded[pos++];
            for (let i = header + 3; i > 0; i--) {
                replayed.push(value);
            }
        } else {
            const numLiterals = 256 - header;
            if (pos + numLiterals > encoded.length) {
                return `a run of ${numLiterals} literals reaches past the end of the stream`;
            }
            for (let i = 0; i < numLiterals; i++) {
                replayed.push(encoded[pos++]);
            }
        }
    }

    if (replayed.length !== expected.length) {
        return `replayed ${replayed.length} bytes, expected ${expected.length}`;
    }
    for (let i = 0; i < expected.length; i++) {
        if (replayed[i] !== expected[i]) {
            return `byte ${i} replayed as ${replayed[i]}, expected ${expected[i]}`;
        }
    }
    return "ok";
}

/** Replays a stream against the headers, then decodes it, and reports the first thing that differs. */
function roundTripByteRle(values: Uint8Array): string {
    const encoded = encodeByteRle(values);
    const replayed = replayByteRle(values, encoded);
    if (replayed !== "ok") {
        return replayed;
    }

    const offset = new IntWrapper(0);
    const decoded = decodeByteRle(encoded, values.length, encoded.length, offset);
    for (let i = 0; i < values.length; i++) {
        if (decoded[i] !== values[i]) {
            return `byte ${i} decoded as ${decoded[i]}, expected ${values[i]}`;
        }
    }
    if (offset.get() !== encoded.length) {
        return `left the offset at ${offset.get()}, expected ${encoded.length}`;
    }
    return "ok";
}

function roundTripBooleanRle(values: boolean[]): string {
    const encoded = encodeBooleanRle(values);
    const offset = new IntWrapper(0);
    const decoded = new BitVector(decodeBooleanRle(encoded, values.length, encoded.length, offset), values.length);

    for (let i = 0; i < values.length; i++) {
        if (decoded.get(i) !== values[i]) {
            return `boolean ${i} decoded as ${decoded.get(i)}, expected ${values[i]}`;
        }
    }
    return "ok";
}

/** Lengths that sit on, just under and just over the points where a header runs out of room. */
const BYTE_LENGTHS = [0, 1, 2, 3, 4, 127, 128, 129, 130, 131, 132, 259, 260, 261, 1023, 1024, 1040, 1041];

const BYTE_PATTERNS: { name: string; byteAt: (index: number) => number }[] = [
    { name: "one repeated byte", byteAt: () => 7 },
    { name: "no two neighbours alike", byteAt: (index) => index % 251 },
    { name: "alternating pair", byteAt: (index) => index % 2 },
    { name: "short runs between literals", byteAt: (index) => (index % 7 < 3 ? 1 : index % 251) },
    { name: "a full literal run then one long run", byteAt: (index) => (index < MAX_LITERALS ? index % 251 : 9) },
];

/** Bit counts around 1024, and around the 1040 booleans that fill the longest byte run. */
const BOOLEAN_LENGTHS = [0, 1, 7, 8, 9, 1016, 1023, 1024, 1025, 1032, 1039, 1040, 1041, 1048, 2080];

const BOOLEAN_PATTERNS: { name: string; valueAt: (index: number) => boolean }[] = [
    { name: "all true", valueAt: () => true },
    { name: "all false", valueAt: () => false },
    { name: "every third", valueAt: (index) => index % 3 === 0 },
    { name: "alternating", valueAt: (index) => index % 2 === 0 },
    { name: "true until the last byte", valueAt: (index) => index < 1032 },
];

describe("encodingUtils", () => {
    describe("encodeByteRle", () => {
        it("should end a run at the longest length a header can express", () => {
            expect(encodeByteRle(new Uint8Array(MAX_RUN).fill(7))).toEqual(new Uint8Array([0x7f, 7]));
            // One byte more than the header can carry needs a second header for the remainder
            expect(encodeByteRle(new Uint8Array(MAX_RUN + 1).fill(7))).toEqual(new Uint8Array([0x7f, 7, 0xff, 7]));
        });

        it("should end a literal run at the longest length a header can express", () => {
            const values = Uint8Array.from({ length: MAX_LITERALS + 1 }, (_, index) => (index * 7 + 1) % 256);

            const expected = new Uint8Array(MAX_LITERALS + 3);
            expected[0] = 256 - MAX_LITERALS;
            expected.set(values.subarray(0, MAX_LITERALS), 1);
            expected[MAX_LITERALS + 1] = 0xff;
            expected[MAX_LITERALS + 2] = values[MAX_LITERALS];

            expect(encodeByteRle(values)).toEqual(expected);
        });

        it("should frame every run and literal so the stream replays back to its input", () => {
            const cases = BYTE_PATTERNS.flatMap(({ name, byteAt }) =>
                BYTE_LENGTHS.map((length) => ({ label: `${name}, ${length} bytes`, length, byteAt })),
            );

            const results = cases.map(
                ({ label, length, byteAt }) =>
                    `${label}: ${roundTripByteRle(Uint8Array.from({ length }, (_, index) => byteAt(index)))}`,
            );

            expect(results).toEqual(cases.map(({ label }) => `${label}: ok`));
        });

        it("should round trip byte streams built from randomly sized runs", () => {
            const random = seededRandom(0x0b17e);
            const failures: string[] = [];

            for (let round = 0; round < 250; round++) {
                const length = Math.floor(random() * 1600);
                const distinctValues = 1 + Math.floor(random() * 5);
                const values = new Uint8Array(length);

                let written = 0;
                while (written < length) {
                    // Run lengths reach well past 130 so that long runs have to be split
                    const value = Math.floor(random() * distinctValues) * 37;
                    const runLength = 1 + Math.floor(random() * 300);
                    for (let i = 0; i < runLength && written < length; i++) {
                        values[written++] = value;
                    }
                }

                const result = roundTripByteRle(values);
                if (result !== "ok") {
                    failures.push(`round ${round}, ${length} bytes: ${result}`);
                }
            }

            expect(failures).toEqual([]);
        });
    });

    describe("encodeBooleanRle", () => {
        it("should pack booleans into bytes before the byte run limit applies", () => {
            // 1040 booleans pack into the 130 bytes that one run header can express
            const oneRun = Array.from({ length: MAX_RUN * 8 }, () => true);
            expect(encodeBooleanRle(oneRun)).toEqual(new Uint8Array([0x7f, 0xff]));

            // 1048 booleans need that run plus a literal for the byte that no longer fits
            const oneRunAndAByte = Array.from({ length: (MAX_RUN + 1) * 8 }, () => true);
            expect(encodeBooleanRle(oneRunAndAByte)).toEqual(new Uint8Array([0x7f, 0xff, 0xff, 0xff]));
        });

        it("should round trip around the bit counts where a byte run ends", () => {
            const cases = BOOLEAN_PATTERNS.flatMap(({ name, valueAt }) =>
                BOOLEAN_LENGTHS.map((length) => ({ label: `${name}, ${length} booleans`, length, valueAt })),
            );

            const results = cases.map(
                ({ label, length, valueAt }) =>
                    `${label}: ${roundTripBooleanRle(Array.from({ length }, (_, index) => valueAt(index)))}`,
            );

            expect(results).toEqual(cases.map(({ label }) => `${label}: ok`));
        });

        it("should round trip randomly generated boolean vectors", () => {
            const random = seededRandom(0x600d);
            const failures: string[] = [];

            for (let round = 0; round < 120; round++) {
                const length = Math.floor(random() * 3000);
                // Vary the bias so rounds range from long uniform runs to noise
                const trueRatio = random();
                const values = Array.from({ length }, () => random() < trueRatio);

                const result = roundTripBooleanRle(values);
                if (result !== "ok") {
                    failures.push(`round ${round}, ${length} booleans: ${result}`);
                }
            }

            expect(failures).toEqual([]);
        });
    });

    describe("present streams", () => {
        it("should round trip a present stream and the values it gates", () => {
            const random = seededRandom(0x5eed);
            const failures: string[] = [];

            for (let round = 0; round < 64; round++) {
                const numFeatures = 1 + Math.floor(random() * 2200);
                const present = Array.from({ length: numFeatures }, () => random() < 0.6);

                const encodedPresent = encodeBooleanRle(present);
                const offset = new IntWrapper(0);
                const presentBits = new BitVector(
                    decodeBooleanRle(encodedPresent, numFeatures, encodedPresent.length, offset),
                    numFeatures,
                );

                const values = Int32Array.from({ length: numFeatures }, () => Math.floor(random() * 2000) - 1000);
                const restored = unpackNullable(packNullable(values, presentBits), presentBits, 0);

                for (let i = 0; i < numFeatures; i++) {
                    const expected = present[i] ? values[i] : 0;
                    if (presentBits.get(i) !== present[i]) {
                        failures.push(`round ${round}: present bit ${i} of ${numFeatures} decoded the other way`);
                        break;
                    }
                    if (restored[i] !== expected) {
                        failures.push(`round ${round}: value ${i} came back as ${restored[i]}, expected ${expected}`);
                        break;
                    }
                }
            }

            expect(failures).toEqual([]);
        });

        it("should round trip a boolean column carried behind a present stream", () => {
            const random = seededRandom(0xb001);
            const failures: string[] = [];

            for (let round = 0; round < 48; round++) {
                const numFeatures = 1 + Math.floor(random() * 2200);
                const present = Array.from({ length: numFeatures }, () => random() < 0.7);
                const values = Array.from({ length: numFeatures }, () => random() < 0.5);

                const presentBits = new BitVector(new Uint8Array(Math.ceil(numFeatures / 8)), numFeatures);
                for (let i = 0; i < numFeatures; i++) {
                    presentBits.set(i, present[i]);
                }

                // The encoders only put the non-null values on the wire, as propertyEncoder does
                const nonNull = values.filter((_, i) => present[i]);
                const encoded = encodeBooleanRle(nonNull);
                const offset = new IntWrapper(0);
                const decoded = new BitVector(
                    decodeBooleanRle(encoded, nonNull.length, encoded.length, offset, presentBits),
                    numFeatures,
                );

                for (let i = 0; i < numFeatures; i++) {
                    const expected = present[i] && values[i];
                    if (decoded.get(i) !== expected) {
                        failures.push(`round ${round}: feature ${i} came back as ${decoded.get(i)}`);
                        break;
                    }
                }
            }

            expect(failures).toEqual([]);
        });
    });

    describe("fixed width streams", () => {
        it("should round trip float, double, uint32 and uint64 streams", () => {
            const random = seededRandom(0xfee1);
            const failures: string[] = [];

            for (const length of [0, 1, 2, 3, 17, 256, 1024]) {
                const floats = Float32Array.from({ length }, () => (random() - 0.5) * 1e6);
                const decodedFloats = decodeFloatsLE(encodeFloatsLE(floats), new IntWrapper(0), length);
                if (String(decodedFloats) !== String(floats)) {
                    failures.push(`floats, ${length} values`);
                }

                const doubles = Float64Array.from({ length }, () => (random() - 0.5) * 1e12);
                const decodedDoubles = decodeDoublesLE(encodeDoubleLE(doubles), new IntWrapper(0), length);
                if (String(decodedDoubles) !== String(doubles)) {
                    failures.push(`doubles, ${length} values`);
                }

                const uint32s = Uint32Array.from({ length }, () => Math.floor(random() * 0x100000000));
                const decodedUint32s = decodeUint32sLE(encodeUint32sLE(uint32s), new IntWrapper(0), length);
                if (String(decodedUint32s) !== String(uint32s)) {
                    failures.push(`uint32s, ${length} values`);
                }

                const uint64s = BigUint64Array.from({ length }, () => {
                    const high = BigInt(Math.floor(random() * 0x100000000));
                    const low = BigInt(Math.floor(random() * 0x100000000));
                    return (high << 32n) | low;
                });
                const decodedUint64s = decodeUint64sLE(encodeUint64sLE(uint64s), new IntWrapper(0), length);
                if (String(decodedUint64s) !== String(uint64s)) {
                    failures.push(`uint64s, ${length} values`);
                }
            }

            expect(failures).toEqual([]);
        });

        it("should round trip strings on both sides of the TextDecoder threshold", () => {
            const strings = [
                "",
                "a",
                "eleven byte",
                "twelve bytes",
                "a rather longer ASCII string",
                "Köln",
                "東京",
                "🌍🗺",
            ];

            const encoded = encodeStrings(strings);
            const lengths = createStringLengths(strings);

            const decoded: string[] = [];
            let offset = 0;
            for (const length of lengths) {
                decoded.push(decodeString(encoded, offset, offset + length));
                offset += length;
            }

            expect(decoded).toEqual(strings);
            expect(offset).toBe(encoded.length);
        });
    });
});
