import {describe, expect, it} from "vitest";
import {decodeFastPfor} from "./fastPforDecoder";
import IntWrapper from "./intWrapper";
import fs from "fs";
import path from 'path';
import {fileURLToPath} from 'url';


const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const TEST_DIR_PATH = path.resolve(__dirname, "../../src/test/data/fastPfor/");

// encoded data was generated using the java fastPfor encoder and expected values are re-generated in the tests
const ENCODED_NON_ALIGNED_358_ENCODED = fs.readFileSync(path.join(TEST_DIR_PATH, 'non-aligned_358.bin'));
const LARGE_EXCEPTIONS_ENCODED = fs.readFileSync(path.join(TEST_DIR_PATH, 'large-exceptions.bin'));
const SEQUENTIAL_ENCODED = fs.readFileSync(path.join(TEST_DIR_PATH, 'sequence.bin'));
const SMALE_VALUES_ENCODED = fs.readFileSync(path.join(TEST_DIR_PATH, 'smale-values.bin'));
const ZEROS_ENCODED = fs.readFileSync(path.join(TEST_DIR_PATH, 'zeros.bin'));
const MULTI_PAGE_ENCODED = fs.readFileSync(path.join(TEST_DIR_PATH, 'multi-page.bin'));
const MAX_EXCEPTIONS_ENCODED = fs.readFileSync(path.join(TEST_DIR_PATH, 'max-exceptions.bin'));
const VARIABLEBYTE_ONLY_ENCODED = fs.readFileSync(path.join(TEST_DIR_PATH, 'variablebyte-only.bin'));
const SINGLE_VB_VALUE_ENCODED = fs.readFileSync(path.join(TEST_DIR_PATH, 'single-vb-value.bin'));
const BIT_WIDTH_32_ENCODED = fs.readFileSync(path.join(TEST_DIR_PATH, 'bit-width-32.bin'));
const ALTERNATING_PATTERN_ENCODED = fs.readFileSync(path.join(TEST_DIR_PATH, 'alternating-pattern.bin'));

describe("FastPFOR Decoder - Java Generated Test Vectors", () => {

    describe("Core Functionality", () => {
        it("should decode non_aligned_358 (256 FastPFOR + 102 VariableByte)", () => {
            const decoded = decodeFastPfor(ENCODED_NON_ALIGNED_358_ENCODED, 358, 480, new IntWrapper(0));

            expect(decoded.length).toBe(358);
            for (let i = 0; i < 358; i++) {
                expect(decoded[i]).toBe(i)
            }
        });

        it("should decode multi-page stream (>65536 values)", () => {
            const decoded = decodeFastPfor(MULTI_PAGE_ENCODED, 66000, 102872, new IntWrapper(0));

            expect(decoded.length).toBe(66000);
            for (let i = 0; i < 66000; i++) {
                expect(decoded[i]).toBe(i % 10000);
            }
        });

        it("should decode sequential (512 values)", () => {
            const decoded = decodeFastPfor(SEQUENTIAL_ENCODED, 512, 564, new IntWrapper(0));

            expect(decoded.length).toBe(512);
            for (let i = 0; i < 512; i++) {
                expect(decoded[i]).toBe(i);
            }
        });
    });

    describe("Exception Handling", () => {
        it("should decode large_exceptions", () => {
            const decoded = decodeFastPfor(LARGE_EXCEPTIONS_ENCODED, 500, 380, new IntWrapper(0));

            expect(decoded.length).toBe(500);
            for (let i = 0; i < 10; i++) {
                expect(decoded[i]).toBe(7)
            }

            // exceptions
            expect(decoded[10]).toBe(100034530);
            expect(decoded[50]).toBe(20000);
            expect(decoded[100]).toBe(30000)
            expect(decoded[499]).toBe(7)
        });

        it("should decode small_values (3-bit wide)", () => {
            const decoded = decodeFastPfor(SMALE_VALUES_ENCODED, 256, 116, new IntWrapper(0));
            for (let i = 0; i < 256; i++) {
                expect(decoded[i]).toBe(i % 8);
            }
        });

        it("should decode block with maximum exceptions (256)", () => {
            const decoded = decodeFastPfor(MAX_EXCEPTIONS_ENCODED, 256, 660, new IntWrapper(0));

            expect(decoded.length).toBe(256);
            for (let i = 0; i < 256; i++) {
                expect(decoded[i]).toBe(1000000 + i);
            }
        });
    });

    describe("Edge Cases", () => {
        it("should decode zeros (all zeros)", () => {
            const decoded = decodeFastPfor(ZEROS_ENCODED, 265, 20, new IntWrapper(0));

            expect(decoded.length).toBe(265);
            expect(Array.from(decoded).every((v) => v === 0)).toBe(true);
        });

        it("should decode VariableByte-only stream (<256 values)", () => {
            const decoded = decodeFastPfor(VARIABLEBYTE_ONLY_ENCODED, 100, 104, new IntWrapper(0));

            expect(decoded.length).toBe(100);
            for (let i = 0; i < 100; i++) {
                expect(decoded[i]).toBe(i);
            }
        });

        it("should decode single VariableByte value (257 total)", () => {
            const decoded = decodeFastPfor(SINGLE_VB_VALUE_ENCODED, 257, 280, new IntWrapper(0));

            expect(decoded.length).toBe(257);
            for (let i = 0; i < 257; i++) {
                expect(decoded[i]).toBe(i);
            }
        });

        it("should decode bit width 32 (maximum bit packing)", () => {
            const decoded = decodeFastPfor(BIT_WIDTH_32_ENCODED, 256, 1012, new IntWrapper(0));

            expect(decoded.length).toBe(256);
            for (let i = 0; i < 256; i++) {
                expect(decoded[i]).toBe(2147483647 - i); // Integer.MAX_VALUE - i
            }
        });

        it("should decode alternating high/low pattern (high exception density)", () => {
            const decoded = decodeFastPfor(ALTERNATING_PATTERN_ENCODED, 512, 1052, new IntWrapper(0));

            expect(decoded.length).toBe(512);
            for (let i = 0; i < 512; i++) {
                if (i % 2 === 0) {
                    expect(decoded[i]).toBe(0);
                } else {
                    expect(decoded[i]).toBe(10000000);
                }
            }
        });
    });

    describe("Offset Handling", () => {
        it("should handle non-zero initial offset", () => {
            // Add padding before the actual data
            const padding = new Uint8Array(32);
            const combined = new Uint8Array(padding.length + SMALE_VALUES_ENCODED.length);
            combined.set(padding, 0);
            combined.set(SMALE_VALUES_ENCODED, padding.length);

            const offset = new IntWrapper(32);
            const decoded = decodeFastPfor(combined, 256, 116, offset);

            expect(decoded.length).toBe(256);
            expect(offset.get()).toBe(32 + Math.ceil(116 / 4) * 4);

            // Verify first few values
            for (let i = 0; i < 10; i++) {
                expect(decoded[i]).toBe(i % 8);
            }
        });

        it("should advance offset correctly for sequential decoding", () => {
            // Combine two encoded streams
            const combined = new Uint8Array(Math.ceil(20 / 4) * 4 + Math.ceil(116 / 4) * 4);
            combined.set(ZEROS_ENCODED, 0);
            combined.set(SMALE_VALUES_ENCODED, Math.ceil(20 / 4) * 4);

            const offset = new IntWrapper(0);

            // Decode first stream
            const decoded1 = decodeFastPfor(ZEROS_ENCODED, 256, 20, offset);
            expect(decoded1.length).toBe(256);
            expect(Array.from(decoded1).every((v) => v === 0)).toBe(true);

            // Decode second stream
            const decoded2 = decodeFastPfor(combined, 256, 116, offset);
            expect(decoded2.length).toBe(256);
            for (let i = 0; i < 10; i++) {
                expect(decoded2[i]).toBe(i % 8);
            }
        });
    });
});
