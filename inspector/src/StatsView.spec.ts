import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import StatsView from "./StatsView.vue";
import { region } from "./testing.ts";

const tree = {
  bufLen: 60,
  regions: [
    region({ offset: 0, len: 40, label: 'layer[0] "roads"', container: true }),
    region({ offset: 0, len: 1, label: "tag", depth: 1 }),
    region({
      offset: 1,
      len: 25,
      label: "geometry",
      depth: 1,
      container: true,
    }),
    region({
      offset: 26,
      len: 14,
      label: 'column[0] OptStr "name"',
      depth: 1,
      container: true,
    }),
    region({ offset: 40, len: 20, label: 'layer[1] "water"', container: true }),
  ],
};

function panel(active: number | null = null) {
  return mount(StatsView, { props: { tree, tile: null, active } });
}

describe("the statistics panel", () => {
  it("totals the tile", () => {
    expect(panel().get("header strong").text()).toBe("60 B");
  });

  it("lists a layer per row", () => {
    expect(
      panel()
        .findAll(".layer .name")
        .map((n) => n.text()),
    ).toEqual(["roads", "water"]);
  });

  it("lists a layer's columns", () => {
    const rows = panel().findAll(".layer")[0].findAll("tbody tr");
    expect(rows.map((r) => r.findAll("td")[0].text())).toEqual([
      "Geometry",
      "name",
    ]);
  });

  it("lights the layer the pointer is over", () => {
    const rows = panel(1).findAll(".layer");
    expect(rows.map((r) => r.classes("active"))).toEqual([false, true]);
  });
});
