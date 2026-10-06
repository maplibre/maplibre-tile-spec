import { describe, expect, it } from "vitest";
import { formatBytes } from "./bytes.ts";

describe("formatBytes", () => {
  it("prints zero as plain bytes", () => {
    expect(formatBytes(0)).toBe("0 B");
  });

  it("keeps 1023 bytes in bytes", () => {
    expect(formatBytes(1023)).toBe("1023 B");
  });

  it("turns 1024 bytes into one KiB", () => {
    expect(formatBytes(1024)).toBe("1.0 KiB");
  });

  it("rounds a fractional KiB to one decimal", () => {
    expect(formatBytes(128967)).toBe("125.9 KiB");
  });

  it("turns 1024 KiB into one MiB", () => {
    expect(formatBytes(1024 * 1024)).toBe("1.0 MiB");
  });

  it("prints five GiB in GiB", () => {
    expect(formatBytes(5 * 1024 ** 3)).toBe("5.0 GiB");
  });

  it("stays in GiB beyond a thousand of them", () => {
    expect(formatBytes(2048 * 1024 ** 3)).toBe("2048.0 GiB");
  });
});
