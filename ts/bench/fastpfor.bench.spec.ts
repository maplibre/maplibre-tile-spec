import { describe, it } from "vitest";

import Benchmark from "benchmark";

import IntWrapper from "../src/decoding/intWrapper";
import { decodeFastPfor, decodeVarintInt32 } from "../src/decoding/integerDecodingUtils";
import { encodeFastPfor, encodeVarintInt32 } from "../src/encoding/integerEncodingUtils";

type Dataset = { name: string; values: Int32Array };

function buildDatasets(includeBig: boolean): Dataset[] {
    const datasets: Dataset[] = [];

    datasets.push({
        name: "sequential_1024",
        values: new Int32Array(Array.from({ length: 1024 }, (_, i) => i)),
    });

    datasets.push({
        name: "small_3bit_4096",
        values: new Int32Array(Array.from({ length: 4096 }, (_, i) => i % 8)),
    });

    {
        const values = new Int32Array(4096);
        values.fill(7);
        values[10] = 100_034_530;
        values[50] = 20_000;
        values[100] = 30_000;
        values[3999] = 1_000_000;
        datasets.push({ name: "exceptions_4096", values });
    }

    {
        const values = new Int32Array(8192);
        let seed = 0x12345678;
        for (let i = 0; i < values.length; i++) {
            seed = (seed * 1103515245 + 12345) >>> 0;
            values[i] = (seed & 0xffff) | 0;
        }
        datasets.push({ name: "random_u16_8192", values });
    }

    if (includeBig) {
        datasets.push({
            name: "multi_page_66000",
            values: new Int32Array(Array.from({ length: 66000 }, (_, i) => i % 10000)),
        });
    }

    return datasets;
}

function formatNumber(n: number): string {
    return new Intl.NumberFormat("en-US", { maximumFractionDigits: 2 }).format(n);
}

describe("bench: fastpfor vs varint decoding", () => {
    it(
        "runs Benchmark.js suites",
        () => {
            const includeBig = process.argv.includes("--big");
            const datasets = buildDatasets(includeBig);

            // eslint-disable-next-line no-console
            console.log("FastPFOR vs Varint decoding benchmarks (TypeScript implementation)");
            // eslint-disable-next-line no-console
            console.log(`Node ${process.version}`);
            // eslint-disable-next-line no-console
            console.log(`Datasets: ${datasets.map((d) => d.name).join(", ")}`);
            // eslint-disable-next-line no-console
            console.log("");

            for (const dataset of datasets) {
                const values = dataset.values;
                const fastPforBytes = encodeFastPfor(values);
                const varintBytes = encodeVarintInt32(values);

                // eslint-disable-next-line no-console
                console.log(`== ${dataset.name} (n=${values.length}) ==`);
                // eslint-disable-next-line no-console
                console.log(`fastpfor bytes: ${fastPforBytes.length}`);
                // eslint-disable-next-line no-console
                console.log(`varint bytes:   ${varintBytes.length}`);

                const fastPforOffset = new IntWrapper(0);
                const varintOffset = new IntWrapper(0);

                const suite = new Benchmark.Suite();
                suite
                    .add("decodeFastPfor", () => {
                        fastPforOffset.set(0);
                        const decoded = decodeFastPfor(fastPforBytes, values.length, fastPforBytes.length, fastPforOffset);
                        if (decoded.length !== values.length) throw new Error("decoded length mismatch");
                        if (fastPforOffset.get() !== fastPforBytes.length) throw new Error("offset mismatch");
                    })
                    .add("decodeVarintInt32", () => {
                        varintOffset.set(0);
                        const decoded = decodeVarintInt32(varintBytes, varintOffset, values.length);
                        if (decoded.length !== values.length) throw new Error("decoded length mismatch");
                        if (varintOffset.get() !== varintBytes.length) throw new Error("offset mismatch");
                    })
                    .on("cycle", (event: any) => {
                        const b = event.target as any;
                        // eslint-disable-next-line no-console
                        console.log(
                            `${b.name}: ${formatNumber(b.hz)} ops/sec ±${formatNumber(b.stats.rme)}% (${b.stats.sample.length} samples)`,
                        );
                    })
                    .on("complete", function (this: any) {
                        const winner = this.filter("fastest").map("name");
                        // eslint-disable-next-line no-console
                        console.log(`fastest: ${winner.join(", ")}`);
                        // eslint-disable-next-line no-console
                        console.log("");
                    })
                    .run({ async: false });
            }
        },
        120_000,
    );
});

