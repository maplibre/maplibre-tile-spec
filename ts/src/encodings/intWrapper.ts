// Ported from https://github.com/lemire/JavaFastPFOR/blob/master/src/main/java/me/lemire/integercompression/IntWrapper.java

export default class IntWrapper {
    constructor(private value: number) {
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
