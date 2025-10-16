export interface IntegerCODEC {
    uncompress(model: {
        input: Uint32Array;
        inpos: number;
        inlength: number;
        output: Uint32Array;
        outpos: number;
    }): void;
}
export interface SkippableIntegerCODEC {
    headlessUncompress(model: {
        input: Uint32Array;
        inpos: number;
        inlength: number;
        output: Uint32Array;
        outpos: number;
        num: number;
    }): void;
}
