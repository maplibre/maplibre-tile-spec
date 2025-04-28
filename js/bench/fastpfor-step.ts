import Benchmark from "benchmark";
import fs from "fs";
import {VarInt} from "../src/encodings/fastpfor/varint";
import {FastPFOR} from "../src/encodings/fastpfor/fastpfor";


const suite = new Benchmark.Suite;


const VARINT_ENCODED_VALUES = new Uint32Array(fs.readFileSync("test/data/250k_step_varint.bin").buffer);
const FASTPFOR_ENCODED_VALUES = new Uint32Array(fs.readFileSync("test/data/250k_step_fastpfor.bin").buffer);

const fastPfor = FastPFOR.default();
const varint = VarInt.default();
const fastpfor_output: Uint32Array = new Uint32Array(250_000);
let varint_output: Uint32Array = new Uint32Array(250_000);

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
        varint_output = varint.uncompress({
            input: VARINT_ENCODED_VALUES,
        });
    })
    .on('cycle', (event: Benchmark.Event) => {
        console.log(String(event.target));
    })
    .on('complete', function () {
        console.log('Fastest is ' + suite.filter('fastest').map('name'));
    })
    .run()
