import { mount } from "@vue/test-utils";
import { describe, expect, it, vi } from "vitest";
import type { BlobInfo, DecodedBlob, DumpTree } from "./annotate.ts";
import RegionDetail from "./RegionDetail.vue";
import { region, TINY_BYTES } from "./testing.ts";

const TYPES: BlobInfo = {
  streamType: "length[var-binary]",
  logical: "none",
  physical: "varint",
  numValues: 1000,
  hint: { kind: "geometryType" },
};

const NAMES = ["Point", "LineString", "Polygon"];

/** Two payloads, so a case can move the selection from one to the other. */
const tree: DumpTree = {
  bufLen: 8,
  regions: [
    region({
      offset: 0,
      len: 4,
      label: "types",
      kind: "dataBlob",
      blob: TYPES,
    }),
    region({ offset: 4, len: 4, label: "more", kind: "dataBlob", blob: TYPES }),
  ],
};

/** What the wasm hands over: the first `max` of `total` values, and the count when it cut any. */
function enumOf(total: number) {
  return vi.fn((_: number, max: number): DecodedBlob => {
    const values = Array.from(
      { length: Math.min(max, total) },
      (_, n) => n % 3,
    );
    return {
      kind: "enum",
      values,
      names: values.map((v) => NAMES[v]),
      truncatedFrom: total > max ? total : null,
    };
  });
}

function detail(decode: (at: number, max: number) => DecodedBlob, index = 0) {
  return mount(RegionDetail, {
    props: { tree, bytes: TINY_BYTES, index, sticky: true, decode },
  });
}

const texts = (pane: ReturnType<typeof detail>, selector: string) =>
  pane.findAll(selector).map((el) => el.text());

describe("a named value", () => {
  it("shows its names, then the numbers they stand for on a line of their own", () => {
    const pane = detail(enumOf(3));
    const [names, raw] = pane.findAll(".values");
    expect(names.findAll("i").map((el) => el.text())).toEqual(NAMES);
    expect(raw.classes()).toContain("raw");
    expect(raw.findAll("i").map((el) => el.text())).toEqual(["0", "1", "2"]);
  });

  it("is not counted twice, since the decoded section counts it", () => {
    const pane = detail(enumOf(3));
    expect(texts(pane, "dt")).not.toContain("values");
    expect(pane.get("h3 small").text()).toBe("3 values");
  });

  it("is named for what it decodes as", () => {
    const pane = detail(enumOf(3));
    const dd = pane.findAll("dd").map((el) => el.text());
    expect(dd).toContain("geometryType");
  });

  it("gives a plain number no second line", () => {
    const pane = detail(() => ({
      kind: "numbers",
      values: [1, 2],
      truncatedFrom: null,
    }));
    expect(pane.findAll(".values")).toHaveLength(1);
  });
});

describe("a value too long to show", () => {
  it("offers the rest, saying how much of it there is", () => {
    const pane = detail(enumOf(1000));
    expect(pane.get(".reveal button").text()).toBe("show 448 more values");
  });

  it("offers only what is left when that is less than a step", () => {
    const pane = detail(enumOf(100));
    expect(pane.get(".reveal button").text()).toBe("show 36 more values");
  });

  it("says one more value, not one more values", () => {
    const pane = detail(enumOf(65));
    expect(pane.get(".reveal button").text()).toBe("show 1 more value");
  });

  it("says one more char for a text one character over", () => {
    const pane = detail(() => ({ kind: "text", value: "a".repeat(257) }));
    expect(pane.get(".reveal button").text()).toBe("show 1 more char");
  });

  it("offers nothing when it all fits", () => {
    const pane = detail(enumOf(10));
    expect(pane.find(".reveal").exists()).toBe(false);
  });

  it("grows by a step on each press, asking the decoder for more rather than faking it", async () => {
    const decode = enumOf(100_000);
    const pane = detail(decode);
    expect(decode).toHaveBeenLastCalledWith(0, 64);

    await pane.get(".reveal button").trigger("click");
    expect(decode).toHaveBeenLastCalledWith(0, 512);
    expect(pane.findAll(".values")[0].findAll("i")).toHaveLength(512);
    expect(pane.get("h3 small").text()).toBe("512 of 100000 values");

    await pane.get(".reveal button").trigger("click");
    expect(decode).toHaveBeenLastCalledWith(0, 4096);
  });

  it("stops growing, so the pane is never asked to draw a million chips", async () => {
    const pane = detail(enumOf(10_000_000));
    for (let n = 0; n < 2; n++) {
      await pane.get(".reveal button").trigger("click");
    }
    expect(pane.findAll(".values")[0].findAll("i")).toHaveLength(4096);
    expect(texts(pane, ".reveal button")).toEqual(["show fewer"]);
  });

  it("goes back to the start on `show fewer`", async () => {
    const decode = enumOf(1000);
    const pane = detail(decode);
    await pane.get(".reveal button").trigger("click");
    expect(pane.findAll(".values")[0].findAll("i")).toHaveLength(512);

    const fewer = pane
      .findAll(".reveal button")
      .find((b) => b.text() === "show fewer");
    await fewer?.trigger("click");
    expect(pane.findAll(".values")[0].findAll("i")).toHaveLength(64);
  });

  it("offers `show fewer` alone once all of it is shown", async () => {
    const pane = detail(enumOf(100));
    await pane.get(".reveal button").trigger("click");
    expect(texts(pane, ".reveal button")).toEqual(["show fewer"]);
  });

  it("starts over on the next payload, which is not the one that was opened up", async () => {
    const decode = enumOf(1000);
    const pane = detail(decode);
    await pane.get(".reveal button").trigger("click");
    expect(decode).toHaveBeenLastCalledWith(0, 512);

    await pane.setProps({ index: 1 });
    expect(decode).toHaveBeenLastCalledWith(1, 64);
    expect(pane.get(".reveal button").text()).toBe("show 448 more values");
  });

  it("counts a long text in characters", async () => {
    const pane = detail(() => ({ kind: "text", value: "a".repeat(1000) }));
    expect(pane.get(".reveal button").text()).toBe("show 744 more chars");

    await pane.get(".reveal button").trigger("click");
    expect(pane.get(".values i").text()).toHaveLength(1000);
    expect(pane.find(".reveal button").text()).toBe("show fewer");
  });
});

describe("repeated values", () => {
  const repeating = (): DecodedBlob => ({
    kind: "enum",
    values: [0, 0, 0, 0, 0, 0, 2, 1],
    names: [
      "Point",
      "Point",
      "Point",
      "Point",
      "Point",
      "Point",
      "Polygon",
      "LineString",
    ],
    truncatedFrom: null,
  });

  it("shows a long run once, with its count", () => {
    const pane = detail(repeating);
    const [names] = pane.findAll(".values");
    expect(names.findAll("i").map((el) => el.text())).toEqual([
      "Point×6",
      "Polygon",
      "LineString",
    ]);
    expect(names.get("b").text()).toBe("×6");
  });

  it("says on the count what it stands for", () => {
    const pane = detail(repeating);
    expect(pane.get(".values b").attributes("title")).toBe(
      "6 identical values in a row",
    );
  });

  it("folds the numbers beneath the same way, so the two lines still line up", () => {
    const pane = detail(repeating);
    const raw = pane.get(".values.raw");
    expect(raw.findAll("i").map((el) => el.text())).toEqual(["0×6", "2", "1"]);
  });

  it("leaves a text payload whole", () => {
    const pane = detail(() => ({ kind: "text", value: "aaaaaaaaaa" }));
    expect(pane.get(".values i").text()).toBe("aaaaaaaaaa");
    expect(pane.find(".values b").exists()).toBe(false);
  });
});

describe("strings", () => {
  const strings = (total: number, max: number): DecodedBlob => ({
    kind: "strings",
    values: Array.from({ length: Math.min(max, total) }, (_, n) => `s${n}`),
    truncatedFrom: total > max ? total : null,
  });

  it("shows each string as a value of its own, counted as strings", () => {
    const pane = detail(() => strings(3, 64));
    expect(pane.findAll(".values i").map((el) => el.text())).toEqual([
      "s0",
      "s1",
      "s2",
    ]);
    expect(pane.get("h3 small").text()).toBe("3 strings");
  });

  it("does not also count the bytes they were stored in", () => {
    const pane = detail(() => strings(3, 64));
    expect(texts(pane, "dt")).not.toContain("values");
  });

  it("offers the rest in strings", () => {
    const pane = detail((_, max) => strings(100, max));
    expect(pane.get(".reveal button").text()).toBe("show 36 more strings");
  });

  it("says one more string, not one more strings", () => {
    const pane = detail((_, max) => strings(65, max));
    expect(pane.get(".reveal button").text()).toBe("show 1 more string");
  });
});
