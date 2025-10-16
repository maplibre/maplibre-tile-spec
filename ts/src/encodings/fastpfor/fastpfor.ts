/**
 * This code is based on the work previously done by
 * Daniel Lemire, http://lemire.me/en/
 */

// eslint-disable functional/no-this-expression
// eslint-disable functional/no-class
// eslint-disable functional/prefer-readonly-type

import ByteBuffer from 'bytebuffer';

import { fastunpack } from './bitpacking';
import { arraycopy, greatestMultiple } from './util';
import {type IntegerCODEC, type SkippableIntegerCODEC} from "./codec";

export class FastPFOR implements IntegerCODEC, SkippableIntegerCODEC {
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

    public static default(): FastPFOR {
        return new FastPFOR(FastPFOR.DEFAULT_PAGE_SIZE);
    }

    public headlessUncompress(model: { input: Uint32Array, inpos: number, output: Uint32Array, outpos: number, num: number }) : void {
        const mynvalue = greatestMultiple(model.num, FastPFOR.BLOCK_SIZE);
        const finalout = model.outpos.valueOf() + mynvalue;
        while (model.outpos.valueOf() != finalout) {
            model.num = Math.min(this.pageSize, finalout - model.outpos.valueOf());
            this.decodePage(model);
        }
    }

    public decodePage(model: { input: Uint32Array, inpos: number, output: Uint32Array, outpos: number, num: number }) {
        const initpos = model.inpos.valueOf();
        const wheremeta = model.input[model.inpos];
        model.inpos += 1;

        let inexcept = initpos + wheremeta;
        const bytesize = model.input[inexcept++];
        this.byteContainer.clear();

        for (let i = inexcept; i < inexcept + Math.floor((bytesize + 3) / 4); i++)
            this.byteContainer.writeInt32(model.input[i]);

        // this.byteContainer.writeBytes(new Uint8Array(model.input).slice(inexcept, inexcept + (bytesize + 3) / 4));
        inexcept += Math.floor((bytesize + 3) / 4);

        const bitmap = model.input[inexcept++];
        for (let k = 2; k <= 32; ++k) {
            if ((bitmap & (1 << (k - 1))) != 0) {
                const size = model.input[inexcept++];
                const roudedup = greatestMultiple(size + 31, 32);
                if (this.dataTobePacked[k].length < roudedup)
                    this.dataTobePacked[k] = new Uint32Array(roudedup);
                if (inexcept + roudedup / 32 * k <= model.input.length) {
                    let j = 0;
                    for (; j < size; j += 32) {
                        fastunpack({ input: model.input, inpos: inexcept, output: this.dataTobePacked[k], outpos: j, bit: k });
                        inexcept += k;
                    }
                    const overflow = j - size;
                    inexcept -= Math.floor(overflow * k / 32);
                } else {
                    let j = 0;
                    const buf: Uint32Array = new Uint32Array(roudedup / 32 * k);
                    const initinexcept = inexcept;

                    arraycopy(model.input, inexcept, buf, 0, model.input.length - inexcept);

                    for (; j < size; j += 32) {
                        fastunpack({ input: buf, inpos: inexcept - initinexcept, output: this.dataTobePacked[k], outpos: j, bit: k });
                        inexcept += k;
                    }
                    const overflow = j - size;
                    inexcept -= Math.floor(overflow * k / 32);
                }
            }
        }
        this.dataPointers.fill(0);
        let tmpoutpos = model.outpos.valueOf();
        let tmpinpos = model.inpos.valueOf();
        this.byteContainer.flip();

        for (let run = 0, run_end = model.num / FastPFOR.BLOCK_SIZE; run < run_end; ++run, tmpoutpos += FastPFOR.BLOCK_SIZE) {
            const b = this.byteContainer.readByte();
            const cexcept = this.byteContainer.readByte() & 0xFF;
            for (let k = 0; k < FastPFOR.BLOCK_SIZE; k += 32) {
                fastunpack({ input: model.input, inpos: tmpinpos, output: model.output, outpos: tmpoutpos + k, bit: b });
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

    public uncompress(model: { input: Uint32Array, inpos: number, inlength: number, output: Uint32Array, outpos: number }) {
        if (model.input.length == 0) return;
        const outlength = model.input[model.inpos];
        model.inpos++;
        this.headlessUncompress({ input: model.input, inpos: model.inpos, output: model.output, outpos: model.outpos, num: outlength });
    }
}
