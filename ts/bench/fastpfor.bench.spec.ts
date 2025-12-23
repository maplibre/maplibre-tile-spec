import { describe, it, expect } from "vitest";

import Benchmark from "benchmark";

import IntWrapper from "../src/decoding/intWrapper";
import { decodeFastPfor, decodeVarintInt32 } from "../src/decoding/integerDecodingUtils";
import { encodeFastPfor, encodeVarintInt32 } from "../src/encoding/integerEncodingUtils";

type Dataset = { name: string; values: Int32Array };

function buildDatasets(): Dataset[] {
    const datasets: Dataset[] = [];

    // Sequential values: best case for delta encoding
    datasets.push({
        name: "sequential_65536",
        values: new Int32Array(Array.from({ length: 65536 }, (_, i) => i)),
    });

    // Low entropy (mod 8): best case for bit-packing
    datasets.push({
        name: "low_entropy_65536",
        values: new Int32Array(Array.from({ length: 65536 }, (_, i) => i % 8)),
    });

    // Sparse exceptions: mostly small values with rare large ones
    {
        const values = new Int32Array(65536);
        values.fill(7);
        values[10] = 100_034_530;
        values[100] = 20_000;
        values[1000] = 30_000;
        values[10000] = 1_000_000;
        values[50000] = 500_000;
        datasets.push({ name: "exceptions_65536", values });
    }

    // Random 16-bit values: realistic mixed entropy
    {
        const values = new Int32Array(65536);
        let seed = 0x12345678;
        for (let i = 0; i < values.length; i++) {
            seed = (seed * 1103515245 + 12345) >>> 0;
            values[i] = (seed & 0xffff) | 0;
        }
        datasets.push({ name: "random_u16_65536", values });
    }

    return datasets;
}

function formatNumber(n: number): string {
    return new Intl.NumberFormat("en-US", { maximumFractionDigits: 2 }).format(n);
}

describe("bench: fastpfor vs varint decoding", () => {
    it("runs Benchmark.js suites", () => {
        const datasets = buildDatasets();

        console.log("FastPFOR vs Varint decoding benchmarks (TypeScript implementation)");

        console.log(`Node ${process.version}`);

        console.log(`Datasets: ${datasets.map((d) => d.name).join(", ")}`);

        console.log("");

        for (const dataset of datasets) {
            const values = dataset.values;
            const fastPforBytes = encodeFastPfor(values);
            const varintBytes = encodeVarintInt32(values);

            // Sanity check: verify FastPFOR decode produces original values
            const checkFastPforOffset = new IntWrapper(0);
            const decodedFastPfor = decodeFastPfor(
                fastPforBytes,
                values.length,
                fastPforBytes.length,
                checkFastPforOffset,
            );
            expect(decodedFastPfor.length).toBe(values.length);
            expect(decodedFastPfor[0]).toBe(values[0]);
            expect(decodedFastPfor[values.length - 1]).toBe(values[values.length - 1]);
            expect(checkFastPforOffset.get()).toBe(fastPforBytes.length);

            // Sanity check: verify Varint decode produces original values
            const checkVarintOffset = new IntWrapper(0);
            const decodedVarint = decodeVarintInt32(varintBytes, checkVarintOffset, values.length);
            expect(decodedVarint.length).toBe(values.length);
            expect(decodedVarint[0]).toBe(values[0]);
            expect(decodedVarint[values.length - 1]).toBe(values[values.length - 1]);
            expect(checkVarintOffset.get()).toBe(varintBytes.length);

            console.log(`== ${dataset.name} (n=${values.length}) ==`);

            console.log(`fastpfor bytes: ${fastPforBytes.length}`);

            console.log(`varint bytes:   ${varintBytes.length}`);

            console.log(`compression ratio: ${(varintBytes.length / fastPforBytes.length).toFixed(2)}x`);

            const fastPforOffset = new IntWrapper(0);
            const varintOffset = new IntWrapper(0);

            const suite = new Benchmark.Suite();
            suite
                .add("decodeFastPfor", () => {
                    fastPforOffset.set(0);
                    decodeFastPfor(fastPforBytes, values.length, fastPforBytes.length, fastPforOffset);
                })
                .add("decodeVarintInt32", () => {
                    varintOffset.set(0);
                    decodeVarintInt32(varintBytes, varintOffset, values.length);
                })
                .on("cycle", (event: any) => {
                    const b = event.target;

                    console.log(
                        `${b.name}: ${formatNumber(b.hz)} ops/sec ±${formatNumber(b.stats.rme)}% (${b.stats.sample.length} samples)`,
                    );
                })
                .on("complete", function (this: any) {
                    const winner = this.filter("fastest").map("name");

                    console.log(`fastest: ${winner.join(", ")}`);

                    console.log("");
                })
                .run({ async: false });
        }
    }, 180_000);
});
