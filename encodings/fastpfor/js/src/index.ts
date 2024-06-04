/**
 * This code is based on the work previously done by
 * Daniel Lemire, http://lemire.me/en/
 */

// eslint-disable functional/no-this-expression
// eslint-disable functional/no-class
// eslint-disable functional/prefer-readonly-type

import ByteBuffer from 'bytebuffer';

import { fastunpack } from './BitPacking';
import { arraycopy, greatestMultiple } from './util';

export class FastPFOR {
  static readonly OVERHEAD_OF_EACH_EXCEPT = 8;
  static readonly DEFAULT_PAGE_SIZE = 65536;
  static readonly BLOCK_SIZE = 256;

  pageSize: number;
  dataTobePacked: Uint32Array[] = new Array(33);
  byteContainer: ByteBuffer;

  dataPointers: Uint32Array = new Uint32Array(33).fill(0);


  private constructor(pagesize: number) {
    this.pageSize = pagesize;

    this.byteContainer = new ByteBuffer(3 * this.pageSize
      / FastPFOR.BLOCK_SIZE + this.pageSize, true);

    for (let k = 1; k < this.dataTobePacked.length; ++k)
      this.dataTobePacked[k] = new Uint32Array(this.pageSize / 32 * 4).fill(0);
  }

  public static default() {
     return new FastPFOR(FastPFOR.DEFAULT_PAGE_SIZE);
  }

  public headlessUncompress(model: { input: Uint32Array, inpos: number, output: Uint32Array, outpos: number, mynvalue: number }) {
    let mynvalue = greatestMultiple(model.mynvalue, FastPFOR.BLOCK_SIZE);
    var finalout = model.outpos.valueOf() + mynvalue;
    var inner_model = { input: model.input, inpos: model.inpos, output: model.output, outpos: model.outpos, thissize: 0 };
    while (inner_model.outpos.valueOf() != finalout) {
      inner_model.thissize = Math.min(this.pageSize, finalout - inner_model.outpos.valueOf());
      this.decodePage(inner_model);
    }
    model.output = inner_model.output;
    model.outpos = inner_model.outpos;
  }

  public decodePage(model: { input: Uint32Array, inpos: number, output: Uint32Array, outpos: number, thissize: number }) {
    const initpos = model.inpos.valueOf();
    const wheremeta = model.input[model.inpos];
    model.inpos += 1;

    var inexcept = initpos + wheremeta;
    const bytesize = model.input[inexcept++];
    this.byteContainer.clear();

    for (let i = inexcept; i < inexcept + Math.floor((bytesize + 3) / 4); i++)
      this.byteContainer.writeInt32(model.input[i]);

    // this.byteContainer.writeBytes(new Uint8Array(model.input).slice(inexcept, inexcept + (bytesize + 3) / 4));
    inexcept += Math.floor((bytesize + 3) / 4);

    const bitmap = model.input[inexcept++];
    for (let k = 2; k <= 32; ++k) {
      if ((bitmap & (1 << (k - 1))) != 0) {
        let size = model.input[inexcept++];
        let roudedup = greatestMultiple(size + 31, 32);
        if (this.dataTobePacked[k].length < roudedup)
          this.dataTobePacked[k] = new Uint32Array(roudedup);
        if (inexcept + roudedup / 32 * k <= model.input.length) {
          var j = 0;
          for (; j < size; j += 32) {
            fastunpack(model.input, inexcept, this.dataTobePacked[k], j, k);
            inexcept += k;
          }
          let overflow = j - size;
          inexcept -= Math.floor(overflow * k / 32);
        } else {
          let j = 0;
          let buf: Uint32Array = new Uint32Array(roudedup / 32 * k);
          let initinexcept = inexcept;

          arraycopy(model.input, inexcept, buf, 0, model.input.length - inexcept);

          for (; j < size; j += 32) {
            fastunpack(buf, inexcept - initinexcept, this.dataTobePacked[k], j, k);
            inexcept += k;
          }
          let overflow = j - size;
          inexcept -= Math.floor(overflow * k / 32);
        }
      }
    }
    this.dataPointers.fill(0);
    let tmpoutpos = model.outpos.valueOf();
    let tmpinpos = model.inpos.valueOf();
    this.byteContainer.flip();

    for (let run = 0, run_end = model.thissize / FastPFOR.BLOCK_SIZE; run < run_end; ++run, tmpoutpos += FastPFOR.BLOCK_SIZE) {
      const b = this.byteContainer.readByte();
      const cexcept = this.byteContainer.readByte() & 0xFF;
      for (let k = 0; k < FastPFOR.BLOCK_SIZE; k += 32) {
        fastunpack(model.input, tmpinpos, model.output, tmpoutpos + k, b);
        tmpinpos += b;
      }

      if (cexcept > 0) {
        const maxbits = this.byteContainer.readByte();
        const index = maxbits - b;
        if (index == 1) {
          for (let k = 0; k < cexcept; ++k) {
            const pos = this.byteContainer.readByte() & 0xFF;
            model.output[pos + tmpoutpos] |= 1 << b;
          }
        } else {
          for (let k = 0; k < cexcept; ++k) {
            const pos = this.byteContainer.readByte() & 0xFF;
            const exceptValue = this.dataTobePacked[index][this.dataPointers[index]++];
            model.output[pos + tmpoutpos] |= exceptValue << b;
          }
        }
      }
    }
    model.outpos = tmpoutpos;
    model.inpos = inexcept;
  }

  public uncompress(model: { input: Uint32Array, inpos: number, output: Uint32Array, outpos: number }) {
    if (model.input.length == 0) return;
// Todo: remove model
    const outlength = model.input[model.inpos];
    model.inpos++;
    this.headlessUncompress({ input: model.input, inpos: model.inpos, output: model.output, outpos: model.outpos, mynvalue: outlength });
  }
}
