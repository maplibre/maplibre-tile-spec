import { describe, expect, it } from "vitest";
import { blobChips, blobNote } from "./blob.ts";

describe("blobChips", () => {
  it("prints a number per chip", () => {
    expect(
      blobChips(
        { kind: "numbers", values: [1, -2, 3], truncatedFrom: null },
        8,
      ),
    ).toEqual(["1", "-2", "3"]);
  });

  it("prints a bigint without its suffix", () => {
    expect(
      blobChips({ kind: "bigints", values: [9n], truncatedFrom: null }, 8),
    ).toEqual(["9"]);
  });

  it("prints a bool as a bit", () => {
    expect(
      blobChips(
        { kind: "bools", values: [true, false], truncatedFrom: null },
        8,
      ),
    ).toEqual(["1", "0"]);
  });

  it("keeps a whole text payload in one chip", () => {
    expect(blobChips({ kind: "text", value: "water" }, 8)).toEqual(["water"]);
  });

  it("cuts a text payload to the character budget", () => {
    expect(blobChips({ kind: "text", value: "a".repeat(20) }, 8)).toEqual([
      "aaaaaaaa",
    ]);
  });

  it("counts characters rather than UTF-16 units", () => {
    expect(
      blobChips({ kind: "text", value: "\u{1f5fa}".repeat(4) }, 2),
    ).toEqual(["\u{1f5fa}\u{1f5fa}"]);
  });

  it("measures binary data instead of printing it", () => {
    expect(blobChips({ kind: "binary", len: 17 }, 8)).toEqual([
      "17 binary bytes",
    ]);
  });

  it("carries a decode failure through as its message", () => {
    expect(blobChips({ kind: "error", message: "bad varint" }, 8)).toEqual([
      "bad varint",
    ]);
  });
});

describe("blobNote", () => {
  it("counts the values it showed", () => {
    expect(
      blobNote({ kind: "numbers", values: [1, 2], truncatedFrom: null }, 8),
    ).toBe("2 values");
  });

  it("counts the values it left behind", () => {
    expect(
      blobNote({ kind: "numbers", values: [1, 2], truncatedFrom: 400 }, 8),
    ).toBe("2 of 400 values");
  });

  it("counts a whole text payload's characters", () => {
    expect(blobNote({ kind: "text", value: "water" }, 8)).toBe("text, 5 chars");
  });

  it("counts a cut text payload against its whole length", () => {
    expect(blobNote({ kind: "text", value: "a".repeat(20) }, 8)).toBe(
      "text, 8 of 20 chars",
    );
  });

  it("names binary data by its kind", () => {
    expect(blobNote({ kind: "binary", len: 17 }, 8)).toBe("binary");
  });

  it("says a failed decode is undecodable", () => {
    expect(blobNote({ kind: "error", message: "bad varint" }, 8)).toBe(
      "undecodable",
    );
  });
});
