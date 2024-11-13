import Benchmark from 'benchmark';
import { FastPFOR } from '../src/encodings/fastpfor/index';
import * as varint from 'varint'


const suite = new Benchmark.Suite;

const Varint_Compressed: number[] = [202,198,156,89,8,8,8,8,8,8,8,8,8,8,20,8,8,8,8,8,8,8,4,202,198,156,89,8,8,8,8,8,8,8,8,8,8,20,8,8,8,8,8,8,8,4,202,198,156,89,8,8,8,8,8,8,8,8,8,8,20,8,8,8,8,8,8,8,4,202,198,156,89,8,8,8,8,8,8,8,8,8,8,20,8,8,8,8,8,8,8,4,202,198,156,89,8,8,8,8,8,8,8,8,8,8,20,8,8,8,8,8,8,8,4,202,198,156,89,8,8,8,8,8,8,8,8,8,8,20,8,8,8,8,8,8,8,4,202,198,156,89,8,8,8,8,8,8,8,8,8,8,20,8,8,8,8,8,8,8,4,202,198,156,89,8,8,8,8,8,8,8,8,8,8,20,8,8,8,8,8,8,8,4,202,198,156,89,8,8,8,8,8,8,8,8,8,8,20,8,8,8,8,8,8,8,4,202,198,156,89,8,8,8,8,8,8,8,8,8,8,20,8,8,8,8,8,8,8,4,202,198,156,89,8,8,8,8,8,8,8,8,8,8,20,8,8,8,8,8,8,8,4,202,198,156,89,8,8,8,8,8,8,8,8,8,8,20,8,8,8,8,8,8,8,4,202,198,156,89,8,8,8,8,8,8,8,8,8,8,20,8,8,8,8,8,8,8,4,202,198,156,89,8,8,8,8,8,8,8,8,8,8,20,8,8,8,8,8,8,8,4];
const FastPFOR_Compressed: Uint32Array = new Uint32Array([ 256, 41, 277094666, -1977546686, 554189328, 138547362, -1575975903, 277094664, -2078209502, 554189328, 138547338, 1108386337, 277094664, -2078209886, 554312208, 138547332, 1108380193, 279060744, -2078209982, 554213904, 170004612, 1108378657, 277487880, -1574893502, 554189328, 144838788, 571507745, 277094666, -1977546686, 554189328, 138547362, -1575975903, 277094664, -2078209502, 554189328, 138547338, 1108386337, 277094664, -2078209886, 554312208, 138547332, 1108380193, 16, 1838341, 1346119700, -1601406876, -253966156, 4194304, 13, -1923532518, 1313254556, -1423498410, -925527151, 1692691145, 447902261, -1668458183, 1447970476, -1851054301, 1427 ]);

const numbers: Uint32Array = new Uint32Array(256);

function decodeArray(buffer: number[]) {
  let number = 0;
  let i = 0;
  for (let offset=0; offset < buffer.length; offset += varint.encodingLength(number)) {
    number = varint.decode(buffer, offset);
    numbers[i] = number;
    i++;
  }
}

const core = FastPFOR.default();
const output: Uint32Array = new Uint32Array(256);

suite.add("FastPFOR decompression", function () {
  core.uncompress({
    input: FastPFOR_Compressed,
    inpos: 0,
    output: output,
    outpos: 0
  });
})
  .add("VarInt depression", function() {
    decodeArray(Varint_Compressed);
  })
  .on('cycle', (event: Benchmark.Event) => {
    console.log(String(event.target));
  })
  .on('complete', function() {
    console.log('Fastest is ' + suite.filter('fastest').map('name'));
  })
  .run()
