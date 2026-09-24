import { flushPromises, mount } from "@vue/test-utils";
import { beforeAll, describe, expect, it } from "vitest";
import SourcePicker from "./SourcePicker.vue";
import { stubDialog } from "./testing.ts";

beforeAll(stubDialog);

const index = [
  { name: "point.mlt", directory: "0x01", bytes: 8 },
  { name: "line.mlt", directory: "0x02", bytes: 8 },
];

function picker() {
  return mount(SourcePicker, { props: { index, current: null } });
}

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
