import { mount } from "@vue/test-utils";
import { beforeAll, describe, expect, it } from "vitest";
import type { DecodedBlob, DumpTree } from "./annotate.ts";
import HexdumpView from "./HexdumpView.vue";
import { defaultView } from "./hex.ts";
import {
  region,
  TINY_BYTES,
  tinyTree,
  WRAPPED_BYTES,
  wrappedTree,
} from "./testing.ts";

beforeAll(() => {
  globalThis.ResizeObserver = class {
    observe() {}
    unobserve() {}
    disconnect() {}
  };
});

const decode = (): DecodedBlob => ({
  kind: "numbers",
  values: [1],
  truncatedFrom: null,
});

function view(
  tree: DumpTree = tinyTree(),
  selected: number | null = null,
  colorful = false,
  bytes: Uint8Array = TINY_BYTES,
) {
  return mount(HexdumpView, {
    props: {
      tree,
      bytes,
      decode,
      error: null,
      view: { ...defaultView(), colorful },
      selected,
    },
    attachTo: document.body,
  });
}

function cellsWith(pane: ReturnType<typeof view>, name: string): number[] {
  return pane
    .findAll("td.cell")
    .flatMap((cell, at) => (cell.classes().includes(name) ? [at] : []));
}

/** Twenty-four bytes under one container, which is a row and a half of `defaultView`'s sixteen. */
const WIDE_BYTES = new Uint8Array(24).fill(0x41);
const wideTree: DumpTree = {
  bufLen: 24,
  regions: [
    region({ offset: 0, len: 24, label: "layer[0]", container: true }),
    region({ offset: 0, len: 24, label: "name", depth: 1, value: "water" }),
  ],
};

describe("the hex map", () => {
  it("lights every byte of a selected container", () => {
    expect(view(tinyTree(), 0).findAll("td.on")).toHaveLength(8);
  });

  it("lights only the bytes of a selected leaf", () => {
    expect(view(tinyTree(), 1).findAll("td.on")).toHaveLength(2);
  });

  it("pads a short last row out to the width of a full one", () => {
    const rows = view(wideTree, null, false, WIDE_BYTES).findAll("tr.hexrow");
    const shape = rows.map((row) => [
      row.findAll("td.cell").length,
      row.findAll("td.pad").length,
    ]);
    expect(shape).toEqual([
      [16, 0],
      [8, 8],
    ]);
  });

  it("marks the bytes a bailed-out walk never reached", () => {
    expect(
      view(tinyTree("<unannotated>")).findAll("td.unannotated"),
    ).toHaveLength(5);
  });
});

describe("the ascii column", () => {
  const twins = (pane: ReturnType<typeof view>, selector: string) =>
    pane
      .findAll(selector)
      .flatMap((el, at) => (el.classes().includes("twin") ? [at] : []));

  it("marks the glyph of a hovered hex byte", async () => {
    const pane = view();
    await pane.findAll("td.cell")[3].trigger("mousemove");
    expect(twins(pane, ".glyph")).toEqual([3]);
    expect(twins(pane, "td.cell")).toEqual([3]);
  });

  it("marks the hex byte of a hovered glyph", async () => {
    const pane = view();
    await pane.findAll(".glyph")[5].trigger("mousemove");
    expect(twins(pane, "td.cell")).toEqual([5]);
    expect(twins(pane, ".glyph")).toEqual([5]);
  });

  it("lights the glyphs of the region a hovered glyph belongs to", async () => {
    const pane = view();
    await pane.findAll(".glyph")[1].trigger("mousemove");
    const lit = (selector: string) =>
      pane
        .findAll(selector)
        .flatMap((el, at) => (el.classes().includes("on") ? [at] : []));
    expect(lit(".glyph")).toEqual([0, 1]);
    expect(lit("td.cell")).toEqual([0, 1]);
  });

  it("names the region of a hovered glyph", async () => {
    const pane = view();
    await pane.findAll(".glyph")[2].trigger("mousemove");
    expect(pane.get(".tip .path").text()).toBe("layer[0].geometry.encoding");
  });

  it("marks nothing once the pointer leaves the map", async () => {
    const pane = view();
    await pane.findAll(".glyph")[5].trigger("mousemove");
    await pane.get("table.spacer").trigger("mouseleave");
    expect(twins(pane, "td.cell")).toEqual([]);
    expect(twins(pane, ".glyph")).toEqual([]);
  });
});

describe("byte bands", () => {
  it("rounds a lit run at its first and last byte only", () => {
    const pane = view(tinyTree(), 0);
    expect(cellsWith(pane, "opens")).toEqual([0]);
    expect(cellsWith(pane, "closes")).toEqual([7]);
  });

  it("rounds a lit run at the row ends it outlasts", () => {
    const pane = view(wideTree, 0, false, WIDE_BYTES);
    expect(cellsWith(pane, "opens")).toEqual([0, 16]);
    expect(cellsWith(pane, "closes")).toEqual([15, 23]);
  });

  it("tints a container and the scalars it holds as one block", () => {
    const pane = view(tinyTree(), null, true);
    expect(cellsWith(pane, "b0")).toEqual([0, 1]);
    expect(cellsWith(pane, "b1")).toEqual([2, 3, 4, 5, 6, 7]);
  });

  it("rounds a block off where the next one starts", () => {
    const pane = view(tinyTree(), null, true);
    expect(cellsWith(pane, "opens")).toEqual([0, 2]);
    expect(cellsWith(pane, "closes")).toEqual([1, 7]);
  });

  it("parts a block from the one beside it", () => {
    expect(cellsWith(view(tinyTree(), null, true), "tail")).toEqual([1, 7]);
  });

  it("parts two touching blocks the tint wraps onto one slot", () => {
    const pane = view(wrappedTree(), null, true, WRAPPED_BYTES);
    expect(cellsWith(pane, "b0")).toEqual([5, 6]);
    expect(cellsWith(pane, "opens")).toEqual([0, 1, 2, 3, 4, 5, 6]);
    expect(cellsWith(pane, "closes")).toEqual([0, 1, 2, 3, 4, 5, 6]);
  });

  it("tints nothing while colorful is off", () => {
    const tinted = view()
      .findAll("td.cell")
      .filter((cell) => cell.classes().some((name) => /^b\d$/.test(name)));
    expect(tinted).toHaveLength(0);
  });
});

describe("the detail pane", () => {
  it("names the selected region and its span", () => {
    const pane = view(tinyTree(), 1);
    expect(pane.get("h2").text()).toBe("name");
    expect(pane.get("dd").text()).toBe("0000 ... 0001 (2 B)");
  });

  it("waits for a region before it says anything", () => {
    expect(view().get(".idle").text()).toBe(
      "Hover a byte, or walk the regions with the arrow keys.",
    );
  });
});

describe("a decoded text payload", () => {
  const blob = {
    streamType: "Data(String)",
    logical: "None",
    physical: "None",
    numValues: 1,
    hint: { kind: "bytes" },
  } as const;

  const blobTree: DumpTree = {
    bufLen: 8,
    regions: [
      region({ offset: 0, len: 8, label: "data", kind: "dataBlob", blob }),
    ],
  };

  function pane(value: string) {
    return mount(HexdumpView, {
      props: {
        tree: blobTree,
        bytes: TINY_BYTES,
        decode: (): DecodedBlob => ({ kind: "text", value }),
        error: null,
        view: defaultView(),
        selected: 0,
      },
      attachTo: document.body,
    });
  }

  it("shows a short string whole and counts its characters", () => {
    const view = pane("water");
    expect(view.get(".values i").text()).toBe("water");
    expect(view.get("h3 small").text()).toBe("text, 5 chars");
  });

  it("clips a string dictionary to the preview length", () => {
    const view = pane("a".repeat(9000));
    expect(view.get(".values i").text()).toHaveLength(256);
    expect(view.get("h3 small").text()).toBe("text, 256 of 9000 chars");
  });

  it("counts characters rather than UTF-16 units", () => {
    const view = pane("\u{1f5fa}".repeat(300));
    expect(view.get("h3 small").text()).toBe("text, 256 of 300 chars");
    expect([...view.get(".values i").text()]).toHaveLength(256);
  });
});

describe("the hover tip", () => {
  const hover = (pane: ReturnType<typeof view>, cell: number) =>
    pane
      .findAll("td.cell")
      [cell].trigger("mousemove", { clientX: 120, clientY: 80 });

  it("names the hovered byte by its path", async () => {
    const pane = view();
    await hover(pane, 2);
    expect(pane.get(".tip .path").text()).toBe("layer[0].geometry.encoding");
  });

  it("measures the region the byte belongs to", async () => {
    const pane = view();
    await hover(pane, 2);
    expect(pane.get(".tip .span").text()).toBe("1 B at 0002");
  });

  it("shows the value the region carries", async () => {
    const pane = view();
    await hover(pane, 0);
    expect(pane.get(".tip .value").text()).toBe("water");
  });

  it("sits below and right of the pointer", async () => {
    const pane = view();
    await hover(pane, 0);
    expect(pane.get(".tip").attributes("style")).toBe(
      "left: 134px; top: 94px;",
    );
  });

  it("moves with the pointer inside one region", async () => {
    const pane = view();
    await hover(pane, 0);
    await pane
      .findAll("td.cell")[1]
      .trigger("mousemove", { clientX: 300, clientY: 80 });
    expect(pane.get(".tip").attributes("style")).toBe(
      "left: 314px; top: 94px;",
    );
  });

  it("goes away once the pointer leaves the map", async () => {
    const pane = view();
    await hover(pane, 0);
    await pane.get("table.spacer").trigger("mouseleave");
    expect(pane.find(".tip").exists()).toBe(false);
  });

  it("stays away from a region the tree hovers instead", async () => {
    const pane = view();
    await pane.get(".node button.label").trigger("mouseenter");
    expect(pane.get(".detail h2").text()).toBe("layer[0] click to pin");
    expect(pane.find(".tip").exists()).toBe(false);
  });

  it("spells out the bits of an encoding byte", async () => {
    const bits = {
      bufLen: 1,
      regions: [
        region({
          offset: 0,
          len: 1,
          label: "encoding",
          bits: [
            { hi: 7, lo: 4, raw: 0, meaning: "logical: none" },
            { hi: 3, lo: 0, raw: 1, meaning: "physical: varint" },
          ],
        }),
      ],
    };
    const pane = view(bits, null, false, new Uint8Array([0x01]));
    await hover(pane, 0);
    expect(pane.findAll(".tip .meaning").map((line) => line.text())).toEqual([
      "logical: none",
      "physical: varint",
    ]);
  });

  it("previews the values a blob decodes to", async () => {
    const pane = mount(HexdumpView, {
      props: {
        tree: {
          bufLen: 8,
          regions: [
            region({
              offset: 0,
              len: 8,
              label: "data",
              kind: "dataBlob",
              blob: {
                streamType: "Data(Int)",
                logical: "None",
                physical: "Varint",
                numValues: 9,
                hint: { kind: "u32" },
              },
            }),
          ],
        },
        bytes: TINY_BYTES,
        decode: (): DecodedBlob => ({
          kind: "numbers",
          values: [1, 2, 3],
          truncatedFrom: 9,
        }),
        error: null,
        view: defaultView(),
        selected: null,
      },
      attachTo: document.body,
    });
    await hover(pane, 0);
    expect(pane.get(".tip .value").text()).toBe("1, 2, 3");
    expect(pane.findAll(".tip .span")[1].text()).toBe("3 of 9 values");
  });
});

describe("arrow keys", () => {
  const press = (key: string) =>
    window.dispatchEvent(new KeyboardEvent("keydown", { key }));

  it("walks down to the first leaf", () => {
    const pane = view();
    press("ArrowDown");
    expect(pane.emitted("update:selected")).toEqual([[1]]);
  });

  it("walks down over containers", () => {
    const pane = view(tinyTree(), 1);
    press("ArrowDown");
    expect(pane.emitted("update:selected")).toEqual([[3]]);
  });

  it("walks right the same as down", () => {
    const pane = view(tinyTree(), 1);
    press("ArrowRight");
    expect(pane.emitted("update:selected")).toEqual([[3]]);
  });

  it("walks left the same as up", () => {
    const pane = view(tinyTree(), 3);
    press("ArrowLeft");
    expect(pane.emitted("update:selected")).toEqual([[1]]);
  });

  it("walks up to the last leaf from nothing", () => {
    const pane = view();
    press("ArrowUp");
    expect(pane.emitted("update:selected")).toEqual([[4]]);
  });

  it("stops at the last leaf", () => {
    const pane = view(tinyTree(), 4);
    press("ArrowDown");
    expect(pane.emitted("update:selected")).toBeUndefined();
  });

  it("leaves the arrows to a focused text field", () => {
    const field = document.createElement("input");
    field.type = "search";
    document.body.append(field);
    field.focus();
    const pane = view();
    press("ArrowDown");
    expect(pane.emitted("update:selected")).toBeUndefined();
    field.remove();
  });

  it("walks no regions behind an open dialog", () => {
    const sheet = document.createElement("dialog");
    const button = document.createElement("button");
    sheet.append(button);
    document.body.append(sheet);
    sheet.setAttribute("open", "");
    button.focus();
    const pane = view();
    press("ArrowDown");
    expect(pane.emitted("update:selected")).toBeUndefined();
    sheet.remove();
  });
});

describe("the walker error", () => {
  it("surfaces the message the handle carries", () => {
    const pane = mount(HexdumpView, {
      props: {
        tree: tinyTree("<unannotated>"),
        bytes: TINY_BYTES,
        decode,
        error: "unexpected end of input",
        view: defaultView(),
        selected: null,
      },
    });
    expect(pane.get("[role=alert]").text()).toBe(
      "walk stopped: unexpected end of input - the remaining bytes are <unannotated>",
    );
  });
});
