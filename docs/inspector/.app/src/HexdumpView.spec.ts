import { mount } from "@vue/test-utils";
import { beforeAll, describe, expect, it } from "vitest";
import type { DecodedBlob, DumpTree } from "./annotate.ts";
import HexdumpView from "./HexdumpView.vue";
import { defaultView } from "./hex.ts";
import { TINY_BYTES, tinyTree } from "./testing.ts";

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

function view(tree: DumpTree = tinyTree(), selected: number | null = null) {
  return mount(HexdumpView, {
    props: {
      tree,
      bytes: TINY_BYTES,
      decode,
      error: null,
      view: defaultView(),
      selected,
    },
    attachTo: document.body,
  });
}

describe("the hex map", () => {
  it("lights every byte of a selected container", () => {
    expect(view(tinyTree(), 0).findAll("td.on")).toHaveLength(8);
  });

  it("lights only the bytes of a selected leaf", () => {
    expect(view(tinyTree(), 1).findAll("td.on")).toHaveLength(2);
  });

  it("marks the bytes a bailed-out walk never reached", () => {
    expect(
      view(tinyTree("<unannotated>")).findAll("td.unannotated"),
    ).toHaveLength(5);
  });
});

describe("the detail pane", () => {
  it("names the selected region and its span", () => {
    const pane = view(tinyTree(), 1);
    expect(pane.get("h2").text()).toBe("name");
    expect(pane.get("dd").text()).toBe("00000000 … 00000001 · 2 B");
  });

  it("waits for a region before it says anything", () => {
    expect(view().get(".idle").text()).toBe(
      "Hover a byte, or walk the regions with ↑ / ↓.",
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
      "walk stopped: unexpected end of input — the remaining bytes are <unannotated>",
    );
  });
});
