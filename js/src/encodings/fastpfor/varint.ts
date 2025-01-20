import {IntegerCODEC, SkippableIntegerCODEC} from "./codec";

export class VarInt implements IntegerCODEC, SkippableIntegerCODEC {
    public static default(): VarInt {
        return new VarInt();
    }

    public uncompress(model: { input: Uint32Array; output: Uint32Array }) {
        let s = 0;
        let val = 0;
        let p = 0;
        const finalp = 0 + model.input.length;
        let tmpoutpos = 0;

        for (let v = 0, shift = 0; p < finalp;) {
            val = model.input[p];
            const c: number = (val >>> s) & 0xFF; // Ensure c is treated as a byte
            // Shift to next byte
            s += 8;
            // Shift to next integer if s == 32
            p += (s >> 5);
            // cycle from 31 to 0
            s = s & 31;
            v += ((c & 127) << shift);
            if ((c & 128) === 128) {
                model.output[tmpoutpos++] = v;
                v = 0;
                shift = 0;
            } else {
                shift += 7;
            }
        }
    }

    public headlessUncompress(model: {
        input: Uint32Array;
        inpos: number;
        inlength: number;
        output: Uint32Array;
        outpos: number;
        num: number
    }) {
        let s = 0;
        let val = 0;
        let p = model.inpos;
        let tmpoutpos = model.outpos;
        const finaloutpos = model.num + tmpoutpos;

        for (let v = 0, shift = 0; tmpoutpos < finaloutpos;) {
            val = model.input[p];
            const c = val >>> s;
            // Shift to next byte
            s += 8;
            // Shift to next integer if s == 32
            p += s >> 5;
            // cycle from 31 to 0
            s = s & 31;
            v += ((c & 127) << shift);

            if ((c & 128) === 128) {
                model.output[tmpoutpos++] = v;
                v = 0;
                shift = 0;
            } else {
                shift += 7;
            }
        }

        model.outpos = tmpoutpos;
        model.inpos = p + (s !== 0 ? 1 : 0);
    }
}
