import {type SkippableIntegerCODEC} from "./codec";
import {FastPFOR} from "./fastpfor";
import {VarInt} from "./varint";


// When in doubt use FastPFORDecoder


export class SkippableComposition implements SkippableIntegerCODEC {
    F1: SkippableIntegerCODEC
    F2: SkippableIntegerCODEC

    public constructor(f1: SkippableIntegerCODEC, f2: SkippableIntegerCODEC) {
        this.F1 = f1;
        this.F2 = f2;
    }

    public headlessUncompress(model: {
        input: Uint32Array;
        inpos: number;
        inlength: number;
        output: Uint32Array;
        outpos: number;
        num: number
    }): void {
        const init = model.inpos;
        const num = model.num;

        this.F1.headlessUncompress(model);
        if (model.inpos == init) {
            model.inpos++;
        }
        model.inlength -= model.inpos - init;
        model.num -= model.outpos;

        this.F2.headlessUncompress(model);

        model.num = num;
    }
}

export class FastPFORDecoder {
    codec: SkippableIntegerCODEC

    public constructor(codec: SkippableIntegerCODEC) {
        this.codec = codec;
    }
    public static default(): FastPFORDecoder {
        return new FastPFORDecoder(new SkippableComposition(
            FastPFOR.default(),
            VarInt.default()
        ));
    }

    public uncompress(input: Uint32Array): Uint32Array {
        const output = new Uint32Array(input[0]);

        this.codec.headlessUncompress({
            input: input,
            inpos: 1,
            inlength: input.length - 1,
            output: output,
            outpos: 0,
            num: output.length
        });

        return output;
    }
}
