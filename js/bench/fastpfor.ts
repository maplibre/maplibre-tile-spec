import Benchmark from "benchmark";
import fs from "fs";
import {VarInt} from "../src/encodings/fastpfor/varint";
import {FastPFOR} from "../src/encodings/fastpfor/fastpfor";


const suite = new Benchmark.Suite;

// function decodeVarint(encodedData: Uint8Array, decodedData: Uint32Array) {
//     let offset = 0;
//     let i = 0;
//     while (offset < encodedData.length) {
//         const value = varint.decode(encodedData, offset);
//         decodedData[i++] = value;
//         offset += varint.decode.bytes;
//     }
// }

const VARINT_ENCODED_VALUES = new Uint32Array(fs.readFileSync("test/data/250k_ascending_varint.bin").buffer);
const FASTPFOR_ENCODED_VALUES = new Uint32Array(fs.readFileSync("test/data/250k_ascending_fastpfor.bin").buffer);

const fastPfor = FastPFOR.default();
const varint = VarInt.default();
const fastpfor_output: Uint32Array = new Uint32Array(250_000);
const varint_output: Uint32Array = new Uint32Array(250_000);

suite.add("FastPFOR decoding", function () {
    fastPfor.uncompress({
        input: FASTPFOR_ENCODED_VALUES,
        inpos: 0,
        inlength: FASTPFOR_ENCODED_VALUES.length,
        output: fastpfor_output,
        outpos: 0,
    });
})
    .add("VarInt decoding", function () {
        varint.uncompress({
            input: VARINT_ENCODED_VALUES,
            output: varint_output,
        });
    })
    .on('cycle', (event: Benchmark.Event) => {
        console.log(String(event.target));
    })
    .on('complete', function () {
        console.log('Fastest is ' + suite.filter('fastest').map('name'));
    })
    .run()
