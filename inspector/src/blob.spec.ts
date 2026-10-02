import { describe, expect, it } from "vitest";
import { blobChips, blobHidden, blobNote, blobRaw, runs } from "./blob.ts";

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

  it("prints what a named value stands for rather than its number", () => {
    expect(
      blobChips(
        {
          kind: "enum",
          values: [0, 2],
          names: ["Point", "Polygon"],
          truncatedFrom: null,
        },
        8,
      ),
    ).toEqual(["Point", "Polygon"]);
  });

  it("prints each string as it is", () => {
    expect(
      blobChips(
        { kind: "strings", values: ["ab", "c"], truncatedFrom: null },
        8,
      ),
    ).toEqual(["ab", "c"]);
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

describe("blobRaw", () => {
  it("hands back the numbers a named value was stored as", () => {
    expect(
      blobRaw({
        kind: "enum",
        values: [0, 2],
        names: ["Point", "Polygon"],
        truncatedFrom: null,
      }),
    ).toEqual(["0", "2"]);
  });

  it("has nothing to add for a value that names nothing", () => {
    expect(
      blobRaw({ kind: "numbers", values: [1], truncatedFrom: null }),
    ).toBeNull();
  });
});

describe("blobHidden", () => {
  it("counts the values cut from the end", () => {
    expect(
      blobHidden({ kind: "numbers", values: [1, 2], truncatedFrom: 400 }, 8),
    ).toEqual({ count: 398, unit: "values" });
  });

  it("counts a named value the same way", () => {
    expect(
      blobHidden(
        {
          kind: "enum",
          values: [0],
          names: ["Point"],
          truncatedFrom: 5,
        },
        8,
      ),
    ).toEqual({ count: 4, unit: "values" });
  });

  it("counts the characters cut from a text payload", () => {
    expect(blobHidden({ kind: "text", value: "a".repeat(20) }, 8)).toEqual({
      count: 12,
      unit: "chars",
    });
  });

  it("has nothing to say when nothing was cut", () => {
    expect(
      blobHidden({ kind: "numbers", values: [1], truncatedFrom: null }, 8),
    ).toBeNull();
    expect(blobHidden({ kind: "text", value: "water" }, 8)).toBeNull();
  });

  it("has nothing to say about what cannot be expanded", () => {
    expect(blobHidden({ kind: "binary", len: 17 }, 8)).toBeNull();
    expect(blobHidden({ kind: "error", message: "bad" }, 8)).toBeNull();
  });
});

describe("runs", () => {
  it("folds a long run into one chip and its count", () => {
    expect(runs(["a", "a", "a", "a", "a", "b"])).toEqual([
      { chip: "a", count: 5, from: 0 },
      { chip: "b", count: 1, from: 5 },
    ]);
  });

  it("writes out a run too short to be worth a count", () => {
    expect(runs(["a", "a", "a", "b"]).map((r) => r.count)).toEqual([
      1, 1, 1, 1,
    ]);
  });

  it("keeps the position of each group, for a line of the same values beneath", () => {
    expect(runs(["x", "a", "a", "a", "a", "y"]).map((r) => r.from)).toEqual([
      0, 1, 5,
    ]);
  });

  it("folds each run on its own when a value comes back", () => {
    const rows = runs([..."aaaabaaaa"]);
    expect(rows.map((r) => [r.chip, r.count])).toEqual([
      ["a", 4],
      ["b", 1],
      ["a", 4],
    ]);
  });
});

describe("a single value", () => {
  it("is not pluralised", () => {
    expect(
      blobNote({ kind: "numbers", values: [1], truncatedFrom: null }, 8),
    ).toBe("1 value");
    expect(
      blobNote({ kind: "strings", values: ["a"], truncatedFrom: null }, 8),
    ).toBe("1 string");
  });
});

describe("strings", () => {
  it("are counted as strings, not values", () => {
    expect(
      blobNote({ kind: "strings", values: ["a", "b"], truncatedFrom: 9 }, 8),
    ).toBe("2 of 9 strings");
    expect(
      blobHidden({ kind: "strings", values: ["a"], truncatedFrom: 9 }, 8),
    ).toEqual({ count: 8, unit: "strings" });
  });
});
