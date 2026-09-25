import { flushPromises, mount } from "@vue/test-utils";
import { beforeAll, describe, expect, it } from "vitest";
import type { FixtureEntry } from "./fixtures.ts";
import SourcePicker from "./SourcePicker.vue";
import { stubDialog } from "./testing.ts";

beforeAll(stubDialog);

const index: FixtureEntry[] = [
  {
    name: "point.mlt",
    directory: "0x01",
    bytes: 8,
    facets: { geometry: ["point"], logical: ["rle"] },
  },
  {
    name: "line.mlt",
    directory: "0x02",
    bytes: 8,
    facets: { geometry: ["line-string"], logical: ["alp"] },
  },
];

function picker(filters: string[] = []) {
  return mount(SourcePicker, { props: { index, current: null, filters } });
}

/** Opens the sheet and the section named, which start collapsed. */
async function sheetWith(view: ReturnType<typeof picker>, section: string) {
  await view.get(".open").trigger("click");
  const head = view
    .findAll(".section-head")
    .find((it) => it.text().includes(section));
  await head?.trigger("click");
  return view;
}

/** Chips of one axis row, by the row's label. */
function chips(view: ReturnType<typeof picker>, label: string) {
  const row = view
    .findAll(".facet")
    .find((it) => it.get(".facet-label").text() === label);
  return row?.findAll(".chip") ?? [];
}

const textOf = (chip: { text: () => string }) => chip.text();

/**
 * Opens the URL dialog, puts `address` in it, and presses Load the way a reader would.
 *
 * `requestSubmit`, not a dispatched submit event: the browser runs the field's own
 * validation first and drops the submit when it fails, and a dispatched event does not.
 * Without that, an input type that turns the address away would go unnoticed here.
 */
async function askFor(view: ReturnType<typeof picker>, address: string) {
  await view.findAll(".ways button")[2].trigger("click");
  await view.get("input.address").setValue(address);
  const load = view.get<HTMLButtonElement>("button[type=submit]");
  view.get<HTMLFormElement>("form").element.requestSubmit();
  await flushPromises();
  return { disabled: load.element.disabled, asked: view.emitted("url") };
}

describe("the URL dialog", () => {
  it("asks for the address that was typed", async () => {
    const view = picker();
    const { asked } = await askFor(view, "https://example.org/a.mlt");
    expect(asked).toEqual([["https://example.org/a.mlt"]]);
  });

  /** `type="url"` would turn this away natively, before the submit handler ever ran. */
  it("takes a relative address, which names a tile beside the page", async () => {
    const view = picker();
    const { asked } = await askFor(view, "fixtures/0x02/point.mlt");
    expect(asked).toEqual([[`${location.origin}/fixtures/0x02/point.mlt`]]);
  });

  it("takes one with no scheme, which is how an address is usually written down", async () => {
    const view = picker();
    const { asked } = await askFor(view, "example.org/a.mlt");
    expect(asked).toEqual([[`${location.origin}/example.org/a.mlt`]]);
  });

  it("refuses a scheme that would read off this reader's own machine", async () => {
    const view = picker();
    const { disabled, asked } = await askFor(view, "file:///etc/passwd");
    expect(disabled).toBe(true);
    expect(asked).toBeUndefined();
  });

  it("says why, rather than leaving a dead button", async () => {
    const view = picker();
    await view.findAll(".ways button")[2].trigger("click");
    await view.get("input.address").setValue("file:///etc/passwd");
    expect(view.get(".hint").text()).toContain("Not a web address");
  });

  it("asks for nothing when nothing was typed", async () => {
    const view = picker();
    const { disabled, asked } = await askFor(view, "   ");
    expect(disabled).toBe(true);
    expect(asked).toBeUndefined();
  });

  it("forgets the last address, so the box opens ready for another", async () => {
    const view = picker();
    await askFor(view, "https://example.org/a.mlt");
    await view.findAll(".ways button")[2].trigger("click");
    expect(view.get<HTMLInputElement>("input.address").element.value).toBe("");
  });
});

describe("the filter sheet", () => {
  it("starts with every section collapsed, so the list stays in view", async () => {
    const view = picker();
    await view.get(".open").trigger("click");
    expect(view.findAll(".section-head")).toHaveLength(4);
    expect(view.findAll(".facet")).toHaveLength(0);
  });

  it("offers the whole vocabulary, not just what the index carries", async () => {
    const view = await sheetWith(picker(), "Geometry");
    expect(chips(view, "geometry").map(textOf)).toEqual([
      "point1",
      "line-string1",
      "polygon0",
      "multi-point0",
      "multi-line-string0",
      "multi-polygon0",
    ]);
  });

  /** A value no fixture has is the one worth seeing, and it cannot be clicked into a corner. */
  it("marks a value no fixture carries and refuses the click", async () => {
    const view = await sheetWith(picker(), "Geometry");
    const gap = chips(view, "geometry")[2];
    expect(gap.classes()).toContain("gap");
    expect(gap.attributes("disabled")).toBeDefined();
  });

  it("counts every chip against what is already picked", async () => {
    const view = await sheetWith(picker(["geometry:point"]), "Streams");
    expect(chips(view, "logical").map(textOf)).toContain("rle1");
    // `alp` is in the index, but not on a point tile, so picking it would dead-end.
    expect(chips(view, "logical").map(textOf)).toContain("alp0");
  });

  it("disables a chip that would narrow to nothing, rather than letting it", async () => {
    const view = await sheetWith(picker(["geometry:point"]), "Streams");
    const alp = chips(view, "logical").find((c) => c.text() === "alp0");
    expect(alp?.classes()).toContain("empty");
    expect(alp?.attributes("disabled")).toBeDefined();
  });

  it("names each picked chip so it can be dropped on its own", async () => {
    const view = picker(["geometry:point"]);
    await view.get(".open").trigger("click");
    const chosen = view.findAll(".chosen .chip");
    expect(chosen.map(textOf)).toEqual(["geometry point×"]);
    await chosen[0].trigger("click");
    expect(view.props("filters")).toEqual(["geometry:point"]);
    expect(view.emitted("update:filters")).toEqual([[[]]]);
  });

  it("shows the section holding a pick already open, with a count", async () => {
    const view = picker(["geometry:point"]);
    await view.get(".open").trigger("click");
    const head = view
      .findAll(".section-head")
      .find((it) => it.text().includes("Geometry"));
    expect(head?.attributes("aria-expanded")).toBe("true");
    expect(head?.get(".badge").text()).toBe("1");
  });

  it("offers a chip for a word that is in no file name", async () => {
    const view = picker();
    await view.get(".open").trigger("click");
    await view.get("input.filter").setValue("alp");
    expect(view.findAll(".hits .chip").map(textOf)).toEqual(["logical alp1"]);
  });

  it("counts coverage over the whole index rather than the picks", async () => {
    const view = await sheetWith(picker(["geometry:point"]), "Geometry");
    expect(chips(view, "geometry").map(textOf)).toContain("line-string0");
    await view
      .findAll(".clear")
      .find((it) => it.text().includes("coverage"))
      ?.trigger("click");
    expect(chips(view, "geometry").map(textOf)).toContain("line-string1");
  });
});
