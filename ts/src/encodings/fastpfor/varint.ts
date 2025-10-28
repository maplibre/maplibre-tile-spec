import { type IntegerCODEC, type SkippableIntegerCODEC } from "./codec";
//
// function toNum(low: number, high: number, isSigned: boolean): number {
//     return isSigned ? high * 0x100000000 + (low >>> 0) : ((high >>> 0) * 0x100000000) + (low >>> 0);
// }
//
// function readVarintRemainder(l: number, s: boolean, model: { input: Uint8Array, pos: number }): number {
//     let h: number, b: number;
//
//     b = model.input[model.pos++]; h  = (b & 0x70) >> 4;  if (b < 0x80) return toNum(l, h, s);
//     b = model.input[model.pos++]; h |= (b & 0x7f) << 3;  if (b < 0x80) return toNum(l, h, s);
//     b = model.input[model.pos++]; h |= (b & 0x7f) << 10; if (b < 0x80) return toNum(l, h, s);
//     b = model.input[model.pos++]; h |= (b & 0x7f) << 17; if (b < 0x80) return toNum(l, h, s);
//     b = model.input[model.pos++]; h |= (b & 0x7f) << 24; if (b < 0x80) return toNum(l, h, s);
//     b = model.input[model.pos++]; h |= (b & 0x01) << 31; if (b < 0x80) return toNum(l, h, s);
//
//     throw new Error('Expected varint not more than 10 bytes');
// }

export class VarInt implements IntegerCODEC, SkippableIntegerCODEC {
    public static default(): VarInt {
        return new VarInt();
    }

    private toNum(low: number, high: number, isSigned: boolean): number {
        return isSigned ? high * 0x100000000 + (low >>> 0) : (high >>> 0) * 0x100000000 + (low >>> 0);
    }

    private readVarintRemainder(val: number, isSigned: boolean, model: { buffer: Uint8Array; pos: number }): number {
        let h: number, b: number;

        b = model.buffer[model.pos++];
        h = (b & 0x70) >> 4;
        if (b < 0x80) return this.toNum(val, h, isSigned);
        b = model.buffer[model.pos++];
        h |= (b & 0x7f) << 3;
        if (b < 0x80) return this.toNum(val, h, isSigned);
        b = model.buffer[model.pos++];
        h |= (b & 0x7f) << 10;
        if (b < 0x80) return this.toNum(val, h, isSigned);
        b = model.buffer[model.pos++];
        h |= (b & 0x7f) << 17;
        if (b < 0x80) return this.toNum(val, h, isSigned);
        b = model.buffer[model.pos++];
        h |= (b & 0x7f) << 24;
        if (b < 0x80) return this.toNum(val, h, isSigned);
        b = model.buffer[model.pos++];
        h |= (b & 0x01) << 31;
        if (b < 0x80) return this.toNum(val, h, isSigned);

        throw new Error("Expected varint not more than 10 bytes");
    }

    private uncompress_single(model: { buffer: Uint8Array; pos: number }, isSigned: boolean = false): number {
        let val: number, b: number;

        b = model.buffer[model.pos++];
        val = b & 0x7f;
        if (b < 0x80) return val;
        b = model.buffer[model.pos++];
        val |= (b & 0x7f) << 7;
        if (b < 0x80) return val;
        b = model.buffer[model.pos++];
        val |= (b & 0x7f) << 14;
        if (b < 0x80) return val;
        b = model.buffer[model.pos++];
        val |= (b & 0x7f) << 21;
        if (b < 0x80) return val;
        b = model.buffer[model.pos];
        val |= (b & 0x0f) << 28;

        return this.readVarintRemainder(val, isSigned, model);
    }

    public uncompress(model: { input: Uint32Array }): Uint32Array {
        const output = new Uint32Array(model.input.length);

        const tmp = new Uint8Array(model.input.buffer);
        const inner_model = {
            buffer: tmp,
            pos: 0,
        };

        for (let i = 0; inner_model.pos < model.input.length; i++) {
            output[i] = this.uncompress_single(inner_model, false);
        }

        return output;

        // const input = new Uint8Array(model.input.buffer);
        //
        // for (let i=0; i < model.input.length; i++) {
        //     model.output[i] = this.decode_single({ input: input, pos: i });
        // }
    }

    public headlessUncompress(model: {
        input: Uint32Array;
        inpos: number;
        inlength: number;
        output: Uint32Array;
        outpos: number;
        num: number;
    }) {
        let s = 0;
        let val = 0;
        let p = model.inpos;
        let tmpoutpos = model.outpos;
        const finaloutpos = model.num + tmpoutpos;

        for (let v = 0, shift = 0; tmpoutpos < finaloutpos; ) {
            val = model.input[p];
            const c = val >>> s;
            // Shift to next byte
            s += 8;
            // Shift to next integer if s == 32
            p += s >> 5;
            // cycle from 31 to 0
            s = s & 31;
            v += (c & 127) << shift;

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
