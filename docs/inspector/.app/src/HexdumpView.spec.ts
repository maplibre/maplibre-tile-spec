import { mount } from "@vue/test-utils";
import { beforeAll, describe, expect, it } from "vitest";
import type { DecodedBlob, DumpTree } from "./annotate.ts";
import HexdumpView from "./HexdumpView.vue";
import { type AnnotateMode, defaultView } from "./hex.ts";
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
  annotate: AnnotateMode = "both",
  bytes: Uint8Array = TINY_BYTES,
) {
  return mount(HexdumpView, {
    props: {
      tree,
      bytes,
      decode,
      error: null,
      view: { ...defaultView(), annotate },
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
    const rows = view(wideTree, null, "both", WIDE_BYTES).findAll("tr.hexrow");
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

describe("byte bands", () => {
  it("rounds a lit run at its first and last byte only", () => {
    const pane = view(tinyTree(), 0);
    expect(cellsWith(pane, "opens")).toEqual([0]);
    expect(cellsWith(pane, "closes")).toEqual([7]);
  });

  it("rounds a lit run at the row ends it outlasts", () => {
    const pane = view(wideTree, 0, "both", WIDE_BYTES);
    expect(cellsWith(pane, "opens")).toEqual([0, 16]);
    expect(cellsWith(pane, "closes")).toEqual([15, 23]);
  });

  it("tints a container and the scalars it holds as one block", () => {
    const pane = view(tinyTree(), null, "sections");
    expect(cellsWith(pane, "b0")).toEqual([0, 1]);
    expect(cellsWith(pane, "b1")).toEqual([2, 3, 4, 5, 6, 7]);
  });

  it("rounds a block off where the next one starts", () => {
    const pane = view(tinyTree(), null, "sections");
    expect(cellsWith(pane, "opens")).toEqual([0, 2]);
    expect(cellsWith(pane, "closes")).toEqual([1, 7]);
  });

  it("parts a block from the one beside it", () => {
    expect(cellsWith(view(tinyTree(), null, "sections"), "tail")).toEqual([
      1, 7,
    ]);
  });

  it("parts two touching blocks the tint wraps onto one slot", () => {
    const pane = view(wrappedTree(), null, "sections", WRAPPED_BYTES);
    expect(cellsWith(pane, "b0")).toEqual([5, 6]);
    expect(cellsWith(pane, "opens")).toEqual([0, 1, 2, 3, 4, 5, 6]);
    expect(cellsWith(pane, "closes")).toEqual([0, 1, 2, 3, 4, 5, 6]);
  });

  it("tints nothing while the knob is off", () => {
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
    expect(pane.get("dd").text()).toBe("00000000 ... 00000001 (2 B)");
  });

  it("waits for a region before it says anything", () => {
    expect(view().get(".idle").text()).toBe(
      "Hover a byte, or walk the regions with the arrow keys.",
    );
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
