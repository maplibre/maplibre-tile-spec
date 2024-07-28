import * as fs from 'node:fs';

import test from 'ava';
import * as varint from 'varint';

import { FastPFOR } from '../../index';
import { arraycopy } from '../../util';

const file = fs.readFileSync("src/tests/assets/testdata.json", "utf8");
const testdata = JSON.parse(file);

function encodeArray(nums: number[]): Uint8Array {
  const buffers: Buffer[] = [];

  // Encode each number to a varint and push it to the buffers array
  for (const number of nums) {
    const encoded = varint.encode(number);
    buffers.push(Buffer.from(encoded));
  }

  // Concatenate all buffer parts into a single buffer
  return new Uint8Array(Buffer.concat(buffers));
}
function decodeArray(buffer: Uint8Array, unpacked_size: number) {
  let numbers = new Uint8Array(unpacked_size);
  let number = 0;

  // Decode each varint from the buffer until the end of the buffer is reached
  for (let offset=0, i = 0; offset < buffer.length; offset += varint.encodingLength(number)) {
    number = varint.decode(buffer, offset);
    numbers[i] = number;
    i++;
  }
  return numbers;
}

test("VarInt compress", (t) => {
  var bytes = encodeArray(testdata.Raw.Medium);
  var numbers = decodeArray(bytes, testdata.Raw.Medium.length);

  // console.log(Array(bytes).toString());

  t.deepEqual(new Uint8Array(testdata.Raw.Medium), numbers);
})
test("VarInt decompress", (t) => {
  var numbers = decodeArray(testdata.Varint.Test1, testdata.Raw.Test1.length);

  t.deepEqual(new Uint8Array(testdata.Raw.Test1), numbers);
})
test("VarInt decompress (Test 1)", (t) => {
  var numbers = decodeArray(testdata.Varint.Test1, testdata.Raw.Test1.length);

  t.deepEqual(new Uint8Array(testdata.Raw.Test1), numbers);
})
test("VarInt decompress (Test 2)", (t) => {
  var numbers = decodeArray(testdata.Varint.Test2, testdata.Raw.Test2.length);

  t.deepEqual(new Uint8Array(testdata.Raw.Test2), numbers);
})
test("VarInt decompress (Medium)", (t) => {
  var numbers = decodeArray(testdata.Varint.Medium, testdata.Raw.Medium.length);

  t.deepEqual(new Uint8Array(testdata.Raw.Medium), numbers);
})
test("VarInt decompress (Large)", (t) => {
  var numbers = decodeArray(testdata.Varint.Large, testdata.Raw.Large.length);

  t.deepEqual(new Uint8Array(testdata.Raw.Large), numbers);
})
test("FastPFOR decompress (Test 1)", (t) => {
  let core = FastPFOR.default();

  var output = new Uint32Array(testdata.Raw.Test1.length);

  let model = {
    input: testdata.FastPFOR.Test1,
    inpos: 0,
    output: output,
    outpos: 0,
    inlength: Array(testdata.FastPFOR.Test1).length,
  };
  core.uncompress(model);

  var SmallInput: Uint32Array = new Uint32Array(model.outpos);
  var SmallOutput: Uint32Array = new Uint32Array(model.outpos);

  arraycopy(new Uint32Array(testdata.Raw.Test1), 0, SmallInput, 0, model.outpos);
  arraycopy(output, 0, SmallOutput, 0, model.outpos);

  t.deepEqual(SmallOutput, SmallInput);
});
test("FastPFOR decompress (Test 2)", (t) => {
  let core = FastPFOR.default();

  var output = new Uint32Array(testdata.Raw.Test2.length);

  let model = {
    input: testdata.FastPFOR.Test2,
    inpos: 0,
    output: output,
    outpos: 0,
    inlength: Array(testdata.FastPFOR.Test2).length,
  };
  core.uncompress(model);

  var SmallInput = new Uint32Array(model.outpos);
  var SmallOutput = new Uint32Array(model.outpos);

  arraycopy(new Uint32Array(testdata.Raw.Test2), 0, SmallInput, 0, model.outpos);
  arraycopy(output, 0, SmallOutput, 0, model.outpos);

  t.deepEqual(SmallOutput, SmallInput);
});
test("FastPFOR decompress (Medium)", (t) => {
  let core = FastPFOR.default();

  var output = new Uint32Array(testdata.Raw.Medium.length);

  let model = {
    input: testdata.FastPFOR.Medium,
    inpos: 0,
    output: output,
    outpos: 0,
    inlength: Array(testdata.FastPFOR.Medium).length,
  };
  core.uncompress(model);

  var SmallInput = new Uint32Array(model.outpos);
  var SmallOutput = new Uint32Array(model.outpos);

  arraycopy(new Uint32Array(testdata.Raw.Medium), 0, SmallInput, 0, model.outpos);
  arraycopy(output, 0, SmallOutput, 0, model.outpos);

  t.deepEqual(SmallOutput, SmallInput);
});
test("FastPFOR decompress (Large)", (t) => {
  let core = FastPFOR.default();

  var output = new Uint32Array(testdata.Raw.Large.length);

  let model = {
    input: testdata.FastPFOR.Large,
    inpos: 0,
    output: output,
    outpos: 0,
    inlength: Array(testdata.FastPFOR.Large).length,
  };
  core.uncompress(model);

  var SmallInput = new Uint32Array(model.outpos);
  var SmallOutput = new Uint32Array(model.outpos);

  arraycopy(new Uint32Array(testdata.Raw.Large), 0, SmallInput, 0, model.outpos);
  arraycopy(output, 0, SmallOutput, 0, model.outpos);

  t.deepEqual(SmallOutput, SmallInput);
});
