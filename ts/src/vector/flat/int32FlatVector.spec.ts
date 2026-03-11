import { describe, it, expect } from "vitest";
import { Int32FlatVector } from "./int32FlatVector";

describe("Int32FlatVector", () => {
  it("should construct and return values correctly", () => {
    const data = new Int32Array([10, 20, 30, 40, 50]);
    const vec = new Int32FlatVector("test", data, data.length);

    expect(vec.name).toBe("test");
    expect(vec.size).toBe(5);
    expect(vec.has(0)).toBe(true);
    expect(vec.getValue(0)).toBe(10);
    expect(vec.getValue(2)).toBe(30);
    expect(vec.getValue(4)).toBe(50);
  });
});
