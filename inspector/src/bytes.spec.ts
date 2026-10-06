import { describe, expect, it } from "vitest";
import { formatBytes } from "./bytes.ts";

describe("formatBytes", () => {
  it.each([
    [0, "0 B"],
    [1023, "1023 B"],
    [1024, "1.0 KiB"],
    [128967, "125.9 KiB"],
    [1048576, "1.0 MiB"],
    [5 * 1024 ** 3, "5.0 GiB"],
    [2048 * 1024 ** 3, "2048.0 GiB"],
  ])("%d reads as %s", (len, text) => {
    expect(formatBytes(len)).toBe(text);
  });
});
