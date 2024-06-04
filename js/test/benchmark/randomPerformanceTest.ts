import { decodeString, decodeVarint } from "../../src/decoder/decodingUtils";
const Benchmark = require("benchmark");

const numValues = 100_000;
const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder("utf-8");
const str1 = textEncoder.encode("TestTest");
const str2 = textEncoder.encode("ABCDEFGHIJKLMNOPQRSTUFWXYZ");
const strBuffer = new Uint8Array((str1.length + str2.length) * (numValues / 2));
let offset = 0;
for (let i = 0; i < numValues; i += 2) {
    strBuffer.set(str1, offset);
    offset += str1.length;
    strBuffer.set(str2, offset);
    offset += str2.length;
}

new Benchmark.Suite()
    /*.add("Typed Array", () => {
        const arr = new Uint32Array(numValues);
        for (let i = 0; i < numValues; i++) {
            arr[i] = i;
        }
    })
    .add("Array", () => {
        const arr = [];
        for (let i = 0; i < numValues; i++) {
            arr.push(i);
        }
    })
    .add("Preallocated Array", () => {
        const arr = new Array(numValues);
        //arr[1_00_000] = 10;
        for (let i = 0; i < numValues; i++) {
            arr[i] = i;
        }
    })*/
    .add("TextDecoder", () => {
        let offset = 0;
        const str1Length = str1.length;
        const str2Length = str2.length;
        for (let i = 0; i < numValues; i += 2) {
            const endOffset1 = offset + str1Length;
            const str1 = decodeStringTextDecoder(strBuffer, offset, endOffset1);
            offset = endOffset1 + str2Length;
            const str2 = decodeStringTextDecoder(strBuffer, endOffset1, offset);
        }
    })
    .add("Optimized decoding", () => {
        let offset = 0;
        const str1Length = str1.length;
        const str2Length = str2.length;
        for (let i = 0; i < numValues; i += 2) {
            const endOffset1 = offset + str1Length;
            const str1 = decodeString(strBuffer, offset, endOffset1);
            offset = endOffset1 + str2Length;
            const str2 = decodeString(strBuffer, endOffset1, offset);
        }
    })
    .on("cycle", (event) => console.info(String(event.target)))
    .on("complete", function () {
        /*console.info(Array.from(this));
        const bench: any = Array.from(this);
        const typedArrayBenchmark = bench.find((b) => b.name.includes("Typed Array"));
        const arrayBenchmark = bench.find((b) => b.name.includes("Array"));
        console.log("Typed array mean decoding time: ", typedArrayBenchmark.stats.mean);
        console.log("Array mean decoding time: ", arrayBenchmark.stats.mean);
        console.log("COVT to MVT decoding performance ratio: ", arrayBenchmark.hz / typedArrayBenchmark.hz);*/
    })
    .run();

function decodeStringTextDecoder(buffer: Uint8Array, startOffset: number, endOffset: number): string {
    const stringSlice = buffer.subarray(startOffset, endOffset);
    return textDecoder.decode(stringSlice);
}
