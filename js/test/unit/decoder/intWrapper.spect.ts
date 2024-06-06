import { IntWrapper } from "../../../src/decoder/intWrapper";

describe("IntWrapper", () => {
    it("should wrap an int like it says it can", async () => {
        const intWrapper = new IntWrapper(5);
        expect(intWrapper.get()).toBe(5);
        intWrapper.set(6);
        expect(intWrapper.get()).toBe(6);
        intWrapper.increment();
        expect(intWrapper.get()).toBe(7);
    });
});
