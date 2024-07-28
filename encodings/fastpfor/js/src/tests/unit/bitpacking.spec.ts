import fs from 'node:fs';

import test from 'ava';

import { fastunpack } from '../../bitpacking'

const file = fs.readFileSync("src/tests/assets/testdata.json", "utf8");
const testdata = JSON.parse(file);


test("Bitpacking unpacking (Test 1)", (t) => {
  let packed = new Uint32Array(testdata.Raw.Bitpacking1.length);
  fastunpack(new Uint32Array(testdata.Bitpacking.Test1), 0, packed, 0, 3);
  t.deepEqual(new Uint32Array(testdata.Raw.Bitpacking1), packed);
});
test("Bitpacking unpacking (Test 2)", (t) => {
  let packed = new Uint32Array(testdata.Raw.Bitpacking2.length);
  fastunpack(new Uint32Array(testdata.Bitpacking.Test2), 0, packed, 0, 5);
  t.deepEqual(new Uint32Array(testdata.Raw.Bitpacking2), packed);
});
test("Bitpacking unpacking (Test 3)", (t) => {
  let packed = new Uint32Array(testdata.Raw.Bitpacking3.length);
  fastunpack(new Uint32Array(testdata.Bitpacking.Test3), 0, packed, 0, 6);
  t.deepEqual(new Uint32Array(testdata.Raw.Bitpacking3), packed);
});
