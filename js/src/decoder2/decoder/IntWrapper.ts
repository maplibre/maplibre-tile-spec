// Ported from https://github.com/lemire/JavaFastPFOR/blob/master/src/main/java/me/lemire/integercompression/IntWrapper.java

export class IntWrapper {
    private value: number;

    constructor(private readonly v: number) {
        this.value = v;
    }

    public get(): number {
        return this.value;
    }

    public set(v: number): void {
        this.value = v;
    }

    public increment(): number {
        return this.value++;
    }

    public add(v: number): void {
        this.value += v;
    }

}
