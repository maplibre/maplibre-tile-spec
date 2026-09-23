import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import type { DumpTree } from "./annotate.ts";
import { regionBands } from "./hex.ts";
import RegionTree from "./RegionTree.vue";
import { region, tinyTree, wrappedTree } from "./testing.ts";

function tree(dump: DumpTree = tinyTree(), sections = false) {
  return mount(RegionTree, {
    props: {
      tree: dump,
      activeIndex: null,
      bands: sections ? regionBands(dump) : null,
    },
  });
}

function bar(view: ReturnType<typeof tree>) {
  const button = view.get(".treebar button");
  return {
    label: button.text(),
    disabled: (button.element as HTMLButtonElement).disabled,
    press: () => button.trigger("click"),
  };
}

const flat: DumpTree = {
  bufLen: 2,
  regions: [region({ offset: 0, len: 2, label: "name", value: "water" })],
};

describe("picking a section header", () => {
  it("expands a collapsed section", async () => {
    const view = tree();
    await view.findAll("button.caret")[1].trigger("click");
    expect(view.findAll(".node")).toHaveLength(3);
    await view.findAll("button.label")[2].trigger("click");
    expect(view.findAll(".node")).toHaveLength(5);
  });

  it("leaves an open section open rather than toggling it shut", async () => {
    const view = tree();
    expect(view.findAll(".node")).toHaveLength(5);
    await view.findAll("button.label")[2].trigger("click");
    expect(view.findAll(".node")).toHaveLength(5);
  });

  it("still reports the pick either way", async () => {
    const view = tree();
    await view.findAll("button.caret")[1].trigger("click");
    await view.findAll("button.label")[2].trigger("click");
    await view.findAll("button.label")[2].trigger("click");
    expect(view.emitted("pick")).toEqual([[2], [2]]);
  });
});

describe("the expand and collapse button", () => {
  it("collapses an open tree", async () => {
    const view = tree();
    expect(bar(view).label).toBe("collapse all");
    await bar(view).press();
    expect(view.findAll(".node")).toHaveLength(1);
  });

  it("expands again once collapsed", async () => {
    const view = tree();
    await bar(view).press();
    expect(bar(view).label).toBe("expand all");
    await bar(view).press();
    expect(view.findAll(".node")).toHaveLength(5);
  });

  it("expands first from a half-open tree", async () => {
    const view = tree();
    await view.findAll("button.caret")[1].trigger("click");
    expect(bar(view).label).toBe("expand all");
    await bar(view).press();
    expect(view.findAll(".node")).toHaveLength(5);
  });

  it("is disabled on a tree without containers", () => {
    expect(bar(tree(flat)).disabled).toBe(true);
  });
});

function rowsWith(view: ReturnType<typeof tree>, name: string): number[] {
  return view
    .findAll(".node")
    .flatMap((node, at) => (node.classes().includes(name) ? [at] : []));
}

describe("row bands", () => {
  it("tints a container and the rows it holds as one block", () => {
    const view = tree(tinyTree(), true);
    expect(rowsWith(view, "b0")).toEqual([0, 1]);
    expect(rowsWith(view, "b1")).toEqual([2, 3, 4]);
  });

  it("rounds a block off where the next one starts", () => {
    const view = tree(tinyTree(), true);
    expect(rowsWith(view, "top")).toEqual([0, 2]);
    expect(rowsWith(view, "bottom")).toEqual([1, 4]);
  });

  it("parts a block from the one below it", () => {
    expect(rowsWith(tree(tinyTree(), true), "tail")).toEqual([1, 4]);
  });

  it("parts two neighbouring blocks the tint wraps onto one slot", () => {
    const view = tree(wrappedTree(), true);
    expect(rowsWith(view, "b0")).toEqual([0, 11, 12, 13]);
    expect(rowsWith(view, "top")).toEqual([0, 1, 3, 5, 7, 9, 11, 13]);
  });

  it("parts nothing while the knob is off", () => {
    expect(rowsWith(tree(), "tail")).toEqual([]);
  });
});

function placeRows(view: ReturnType<typeof tree>, rowHeight: number) {
  const scroll = view.get(".scroll").element as HTMLElement;
  scroll.getBoundingClientRect = () => new DOMRect(0, 0, 200, rowHeight * 2);
  for (const node of view.findAll(".node")) {
    const index = Number(node.attributes("data-index"));
    node.element.getBoundingClientRect = () =>
      new DOMRect(0, index * rowHeight - scroll.scrollTop, 200, rowHeight);
  }
  return scroll;
}

describe("revealing a region", () => {
  it("scrolls a row below the fold to the top", async () => {
    const view = tree();
    const scroll = placeRows(view, 10);
    await view.vm.reveal(4);
    expect(scroll.scrollTop).toBe(40);
  });

  it("leaves a row already on screen where it is", async () => {
    const view = tree();
    const scroll = placeRows(view, 10);
    await view.vm.reveal(1);
    expect(scroll.scrollTop).toBe(0);
  });

  it("opens a collapsed container to reach the row", async () => {
    const view = tree();
    await bar(view).press();
    await view.vm.reveal(4);
    expect(view.findAll(".node")).toHaveLength(5);
  });
});
