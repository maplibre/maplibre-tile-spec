import { flushPromises, mount } from "@vue/test-utils";
import {
  afterEach,
  beforeAll,
  beforeEach,
  describe,
  expect,
  it,
  vi,
} from "vitest";
import App from "./App.vue";
import { type AnnotatedTile, annotateTile } from "./annotate.ts";
import SourcePicker from "./SourcePicker.vue";
import { stubDialog, tinyTree } from "./testing.ts";

vi.mock("./annotate.ts", () => ({ annotateTile: vi.fn() }));

const index = [
  { name: "point.mlt", directory: "0x01", bytes: 8 },
  { name: "line.mlt", directory: "0x02", bytes: 8 },
];

function handle(error: string | null = null): AnnotatedTile {
  const tree = tinyTree(error === null ? "data" : "<unannotated>");
  return {
    tree: () => tree,
    error,
    decodeBlob: () => ({ kind: "numbers", values: [1], truncatedFrom: null }),
    free: () => {},
  };
}

function serve(tile: Response | null = new Response(new Uint8Array(8))) {
  const fetched = vi.fn(async (url: string) => {
    if (url === "fixtures.json") return Response.json(index);
    return tile ?? new Response(null, { status: 404, statusText: "Not Found" });
  });
  vi.stubGlobal("fetch", fetched);
  return fetched;
}

beforeAll(() => {
  globalThis.ResizeObserver = class {
    observe() {}
    unobserve() {}
    disconnect() {}
  };
  globalThis.matchMedia = () =>
    ({
      matches: false,
      addEventListener: () => {},
      removeEventListener: () => {},
    }) as unknown as MediaQueryList;
  stubDialog();
});

beforeEach(() => {
  history.replaceState(null, "", "/");
  vi.mocked(annotateTile).mockReturnValue(handle());
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("the empty state", () => {
  it("asks for a tile before one is loaded", async () => {
    serve();
    const app = mount(App);
    await flushPromises();
    expect(app.get(".empty h1").text()).toBe("Inspect MLT internals");
  });

  it("counts the catalogue on the button that opens the sheet", async () => {
    serve();
    const app = mount(App);
    await flushPromises();
    expect(app.get("button.open").text()).toBe(
      "Browse one of 2 synthetic fixtures",
    );
  });

  it("omits the count while the index is still in flight", async () => {
    serve();
    const app = mount(App);
    expect(app.get("button.open").text()).toBe("Browse the synthetic fixtures");
    await flushPromises();
  });

  it("hides the starter cards when the index carries none of them", async () => {
    serve();
    const app = mount(App);
    await flushPromises();
    expect(app.find(".starters").exists()).toBe(false);
  });

  it("loads the tile a starter card names", async () => {
    const fetched = vi.fn(async (url: string) =>
      url === "fixtures.json"
        ? Response.json([{ name: "point.mlt", directory: "0x02", bytes: 21 }])
        : new Response(new Uint8Array(8)),
    );
    vi.stubGlobal("fetch", fetched);
    const app = mount(App);
    await flushPromises();
    expect(app.get(".starter .title").text()).toBe("a single point");
    await app.get("button.starter").trigger("click");
    await flushPromises();
    expect(fetched).toHaveBeenCalledWith("fixtures/0x02/point.mlt");
    expect(app.get("button.open").text()).toBe("0x02/point.mlt");
  });

  it("offers every indexed fixture once the sheet is open", async () => {
    serve();
    const app = mount(App);
    await flushPromises();
    await app.get("button.open").trigger("click");
    expect(app.findAll("dialog h3").map((group) => group.text())).toEqual([
      "0x01/point",
      "0x02/line",
    ]);
  });

  it("narrows the sheet to the fixtures the filter names", async () => {
    serve();
    const app = mount(App);
    await flushPromises();
    await app.get("button.open").trigger("click");
    await app.get("dialog .filter").setValue("0x02");
    expect(app.findAll("dialog .entry .name").map((e) => e.text())).toEqual([
      "line.mlt",
    ]);
  });

  it("finds a fixture from letters spread over its key", async () => {
    serve();
    const app = mount(App);
    await flushPromises();
    await app.get("button.open").trigger("click");
    await app.get("dialog .filter").setValue("ln");
    expect(app.findAll("dialog .entry .name").map((e) => e.text())).toEqual([
      "line.mlt",
    ]);
  });

  it("says so when the filter matches no fixture", async () => {
    serve();
    const app = mount(App);
    await flushPromises();
    await app.get("button.open").trigger("click");
    await app.get("dialog .filter").setValue("nothing");
    expect(app.get("dialog .none").text()).toBe("no fixture matches");
  });

  it("loads the fixture the sheet picks and closes it", async () => {
    serve();
    const app = mount(App);
    await flushPromises();
    await app.get("button.open").trigger("click");
    await app.get("dialog .entry").trigger("click");
    await flushPromises();
    expect(annotateTile).toHaveBeenCalledOnce();
    expect(app.get<HTMLDialogElement>("dialog").element.open).toBe(false);
    expect(app.get("button.open").text()).toBe("0x01/point.mlt");
  });
});

describe("the sheet's quick filters", () => {
  async function sheet() {
    serve();
    const app = mount(App);
    await flushPromises();
    await app.get("button.open").trigger("click");
    return app;
  }

  const names = (app: Awaited<ReturnType<typeof sheet>>) =>
    app.findAll("dialog .entry .name").map((entry) => entry.text());

  it("counts what each chip would leave", async () => {
    const app = await sheet();
    expect(app.get("dialog .tag[data-tag=v1]").text()).toBe("v1 1");
    expect(app.get("dialog .tag[data-tag=polygon]").text()).toBe("polygon 0");
  });

  it("counts the fixtures the sheet is showing", async () => {
    const app = await sheet();
    expect(app.get("dialog h2 small").text()).toBe("2 of 2");
  });

  it("narrows the sheet to a picked version", async () => {
    const app = await sheet();
    await app.get("dialog .tag[data-tag=v2]").trigger("click");
    expect(names(app)).toEqual(["line.mlt"]);
    expect(app.get("dialog h2 small").text()).toBe("1 of 2");
  });

  it("ors two chips of one facet", async () => {
    const app = await sheet();
    await app.get("dialog .tag[data-tag=v1]").trigger("click");
    await app.get("dialog .tag[data-tag=v2]").trigger("click");
    expect(names(app)).toEqual(["point.mlt", "line.mlt"]);
  });

  it("unpicks a chip that is picked", async () => {
    const app = await sheet();
    await app.get("dialog .tag[data-tag=v2]").trigger("click");
    await app.get("dialog .tag[data-tag=v2]").trigger("click");
    expect(names(app)).toEqual(["point.mlt", "line.mlt"]);
  });

  it("ands a chip onto the filter box", async () => {
    const app = await sheet();
    await app.get("dialog .filter").setValue("l");
    expect(names(app)).toEqual(["point.mlt", "line.mlt"]);
    await app.get("dialog .tag[data-tag=v2]").trigger("click");
    expect(names(app)).toEqual(["line.mlt"]);
  });

  it("stops a chip the filter box has emptied", async () => {
    const app = await sheet();
    await app.get("dialog .filter").setValue("point");
    expect(
      app.get<HTMLButtonElement>("dialog .tag[data-tag=v2]").element.disabled,
    ).toBe(true);
  });

  it("stops a chip with nothing left to narrow", async () => {
    const app = await sheet();
    expect(
      app.get<HTMLButtonElement>("dialog .tag[data-tag=polygon]").element
        .disabled,
    ).toBe(true);
  });

  it("clears every picked chip at once", async () => {
    const app = await sheet();
    await app.get("dialog .tag[data-tag=v2]").trigger("click");
    await app.get("dialog .clear").trigger("click");
    expect(names(app)).toEqual(["point.mlt", "line.mlt"]);
    expect(app.find("dialog .clear").exists()).toBe(false);
  });
});

describe("loading a fixture", () => {
  it("annotates the tile the deep link names", async () => {
    serve();
    history.replaceState(null, "", "/?fixture=0x01/point.mlt");
    const app = mount(App);
    await flushPromises();
    expect(annotateTile).toHaveBeenCalledOnce();
    expect(app.find(".empty").exists()).toBe(false);
  });

  it("reports a fixture that does not load", async () => {
    serve(null);
    history.replaceState(null, "", "/?fixture=0x01/gone.mlt");
    const app = mount(App);
    await flushPromises();
    expect(app.get("[role=alert]").text()).toBe(
      "Error: 0x01/gone.mlt: 404 Not Found",
    );
  });

  it("annotates an uploaded tile", async () => {
    serve();
    const app = mount(App);
    await flushPromises();
    app
      .findComponent(SourcePicker)
      .vm.$emit("upload", new File([new Uint8Array(8)], "own.mlt"));
    await flushPromises();
    expect(annotateTile).toHaveBeenCalledOnce();
    expect(app.find(".empty").exists()).toBe(false);
  });
});

/** jsdom has no DragEvent, and the drop zone reads only `items` and `files` off the transfer. */
function dragging(kind: string, file: File | null): Event {
  const event = new Event(kind, { bubbles: true });
  Object.defineProperty(event, "dataTransfer", {
    value: {
      items: file === null ? [] : [{ kind: "file", type: file.type }],
      files: file === null ? [] : [file],
    },
  });
  return event;
}

describe("a dropped tile", () => {
  it("marks the app while a tile is over it", async () => {
    serve();
    const app = mount(App);
    await flushPromises();
    app.element.dispatchEvent(dragging("dragenter", new File([], "own.mlt")));
    await flushPromises();
    expect(app.classes()).toContain("dragging");
  });

  it("annotates the tile the drop carries and unmarks the app", async () => {
    serve();
    const app = mount(App);
    await flushPromises();
    const file = new File([new Uint8Array(8)], "own.mlt");
    app.element.dispatchEvent(dragging("dragenter", file));
    await flushPromises();
    expect(app.classes()).toContain("dragging");
    app.element.dispatchEvent(dragging("drop", file));
    await flushPromises();
    expect(annotateTile).toHaveBeenCalledOnce();
    expect(app.classes()).not.toContain("dragging");
  });

  it("stays put when the drop carries no file", async () => {
    serve();
    const app = mount(App);
    await flushPromises();
    app.element.dispatchEvent(dragging("drop", null));
    await flushPromises();
    expect(annotateTile).not.toHaveBeenCalled();
  });
});

describe("the deep link", () => {
  it("carries the fixture and the selected region", async () => {
    serve();
    history.replaceState(null, "", "/?fixture=0x01/point.mlt");
    const app = mount(App);
    await flushPromises();
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown" }));
    await flushPromises();
    expect(location.search).toBe("?fixture=0x01%2Fpoint.mlt&region=1");
    app.unmount();
  });

  it("drops a region an upload cannot name", async () => {
    serve();
    history.replaceState(null, "", "/?fixture=0x01/point.mlt&region=1");
    const app = mount(App);
    await flushPromises();
    app
      .findComponent(SourcePicker)
      .vm.$emit("upload", new File([new Uint8Array(8)], "own.mlt"));
    await flushPromises();
    expect(location.search).toBe("");
  });
});

describe("a tile the walker could not finish", () => {
  it("surfaces the error beside the partial tree", async () => {
    serve();
    vi.mocked(annotateTile).mockReturnValue(handle("unexpected end of input"));
    history.replaceState(null, "", "/?fixture=0x01/point.mlt");
    const app = mount(App);
    await flushPromises();
    expect(app.get("[role=alert]").text()).toBe(
      "walk stopped: unexpected end of input - the remaining bytes are <unannotated>",
    );
  });
});
