import fs from 'node:fs';

import Benchmark from 'benchmark';
import { encode } from 'varint';
import * as varint from 'varint'
import 'ava';

import { FastPFOR } from '../../index';


const file = fs.readFileSync("src/tests/assets/testdata.json", "utf8");
const testdata = JSON.parse(file);


const suite_test1 = new Benchmark.Suite;
const suite_test2 = new Benchmark.Suite;
const suite_medium = new Benchmark.Suite;
const suite_large = new Benchmark.Suite;


function encodeArray(input: Uint32Array): Uint8Array {
  const encoded: Uint8Array = new Uint8Array(input.reduce((total, num) => total + varint.encodingLength(num), 0));
  let offset = 0;

  for (const num of input) {
    const bytes = varint.encode(num, encoded, offset);
    offset += bytes.length;
  }

  return encoded;
}
function decodeArray(input: Uint8Array): Uint32Array {
  const decoded = new Uint32Array(input.length);
  let offset = 0;
  let index = 0;

  while (offset < input.length) {
    decoded[index++] = varint.decode(input, offset);
    offset += varint.decode.bytes;
  }

  return decoded.subarray(0, index);
}




const core = FastPFOR.default();

suite_test1.add("FastPFOR decompression (Test 1)", function () {
  const output: Uint32Array = new Uint32Array(testdata.Raw.Test1.length);
  core.uncompress({
    input: testdata.FastPFOR.Test1,
    inpos: 0,
    output: output,
    outpos: 0
  });
})
  .add("VarInt decompression  (Test 1)", function() {
    try {
      decodeArray(testdata.Varint.Test1);
    } catch (_) {
      console.log("ERROR!!!");
    }
  })
  .on('cycle', (event: Benchmark.Event) => {
    console.log("[*] " + String(event.target));
  })
  .on('complete', function() {
    console.log('[+] Fastest is ' + suite_test1.filter('fastest').map('name'));

    // let coded: Uint8Array = encodeArray(testdata.Raw.Large);
    // fs.writeFile("coded.txt", coded.toString(), function (err) { console.log("Failed to write (cod) => " + err) });


    // let data: Uint32Array = readNumbersFromFile("C:\\Users\\BOEH_THO\\Desktop\\in.txt");
    //
    // let enc_data = encodeArray(data);
    //
    // writeNumbersToFile("C:\\Users\\BOEH_THO\\Desktop\\out.txt", enc_data);
  })
  .run();

suite_test2.add("FastPFOR decompression (Test 2)", function () {
  const output: Uint32Array = new Uint32Array(testdata.Raw.Test2.length);
  core.uncompress({
    input: testdata.FastPFOR.Test2,
    inpos: 0,
    output: output,
    outpos: 0
  });
})
  .add("VarInt decompression  (Test 2)", function() {
    try {
      decodeArray(testdata.Varint.Test2);
    } catch (_) {
      console.log("ERROR!!!");
    }
  })
  .on('cycle', (event: Benchmark.Event) => {
    console.log("[*] " + String(event.target));
  })
  .on('complete', function() {
    console.log('[+] Fastest is ' + suite_test2.filter('fastest').map('name'));
  })
  .run();

suite_medium.add("FastPFOR decompression (Medium)", function () {
  const output: Uint32Array = new Uint32Array(testdata.Raw.Medium.length);
  core.uncompress({
    input: testdata.FastPFOR.Medium,
    inpos: 0,
    output: output,
    outpos: 0
  });
})
  .add("VarInt decompression (Medium)", function() {
    try {
      decodeArray(testdata.Varint.Medium);
    } catch (_) {
      console.log("ERROR!!!");
    }
  })
  .on('cycle', (event: Benchmark.Event) => {
    console.log("[*] " + String(event.target));
  })
  .on('complete', function() {
    console.log('[+] Fastest is ' + suite_medium.filter('fastest').map('name'));
  })
  .run();

suite_large.add("FastPFOR decompression (Large)", function () {
  const output: Uint32Array = new Uint32Array(testdata.Raw.Large.length);
  core.uncompress({
    input: testdata.FastPFOR.Large,
    inpos: 0,
    output: output,
    outpos: 0
  });
})
  .add("VarInt decompression (Large)", function() {
    try {
      decodeArray(testdata.Varint.Large);
    } catch (_) {
      console.log("ERROR!!!");
    }
  })
  .on('cycle', (event: Benchmark.Event) => {
    console.log("[*] " + String(event.target));
  })
  .on('complete', function() {
    console.log('[+] Fastest is ' + suite_large.filter('fastest').map('name'));
  })
  .run();
