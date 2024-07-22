import fs from 'node:fs';

import Benchmark from 'benchmark';
import * as varint from 'varint'
import 'ava';

import { FastPFOR } from '../../index';


const file = fs.readFileSync("src/tests/assets/testdata.json", "utf8");
const testdata = JSON.parse(file);


const suite_test1 = new Benchmark.Suite;
const suite_test2 = new Benchmark.Suite;
const suite_medium = new Benchmark.Suite;
const suite_large = new Benchmark.Suite;


function encodeArray(buffer: Uint8Array) {
  const numbers: Uint32Array = new Uint32Array(buffer.length);

  for (let offset= 0, i = 0; offset < buffer.length; offset += varint.encodingLength(buffer[i]), i++) {
    let number = varint.encode(buffer[i]);
    for (let j = 0; j < varint.encodingLength(buffer[i]); j++)
      numbers[offset] = number[j];
  }

  return numbers;
}
function decodeArray(buffer: Uint8Array, unpacked_size: number) {
  const numbers: Uint32Array = new Uint32Array(unpacked_size);
  let number = 0;

  for (let offset= 0, i = 0; offset < buffer.length; offset += varint.encodingLength(number)) {
    number = varint.decode(buffer, offset);
    numbers[i] = number;
    i++;
  }

  return numbers;
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
    decodeArray(testdata.Varint.Test1, testdata.Raw.Test1.length);
  })
  .on('cycle', (event: Benchmark.Event) => {
    console.log(String(event.target));
  })
  .on('complete', function() {
    console.log('Fastest is ' + suite_test1.filter('fastest').map('name'));
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
    decodeArray(testdata.Varint.Test2, testdata.Raw.Test2.length);
  })
  .on('cycle', (event: Benchmark.Event) => {
    console.log(String(event.target));
  })
  .on('complete', function() {
    console.log('Fastest is ' + suite_test2.filter('fastest').map('name'));
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
    decodeArray(testdata.Varint.Medium, testdata.Raw.Medium.length);
  })
  .on('cycle', (event: Benchmark.Event) => {
    console.log(String(event.target));
  })
  .on('complete', function() {
    console.log('Fastest is ' + suite_medium.filter('fastest').map('name'));
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
    decodeArray(testdata.Varint.Large, testdata.Raw.Large.length);
  })
  .on('cycle', (event: Benchmark.Event) => {
    console.log(String(event.target));
  })
  .on('complete', function() {
    console.log('Fastest is ' + suite_large.filter('fastest').map('name'));
  })
  .run();
