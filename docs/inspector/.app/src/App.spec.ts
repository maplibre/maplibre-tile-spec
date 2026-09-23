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

describe("the home button", () => {
  it("is absent until a tile is loaded", async () => {
    serve();
    const app = mount(App);
    await flushPromises();
    expect(app.find("button.home").exists()).toBe(false);
  });

  it("returns a loaded tile to the empty state and clears the deep link", async () => {
    serve();
    const app = mount(App);
    await flushPromises();
    await app.getComponent(SourcePicker).vm.$emit("fixture", "0x01/point.mlt");
    await flushPromises();
    expect(app.find(".empty").exists()).toBe(false);
    expect(location.search).toBe("?fixture=0x01%2Fpoint.mlt");

    await app.get("button.home").trigger("click");
    await flushPromises();
    expect(app.get(".empty h1").text()).toBe("MapLibre Tile Analyzer");
    expect(location.search).toBe("");
  });

  it("frees the tile it drops", async () => {
    serve();
    const free = vi.fn();
    vi.mocked(annotateTile).mockReturnValue({ ...handle(), free });
    const app = mount(App);
    await flushPromises();
    await app.getComponent(SourcePicker).vm.$emit("fixture", "0x01/point.mlt");
    await flushPromises();
    await app.get("button.home").trigger("click");
    expect(free).toHaveBeenCalledOnce();
  });
});

describe("the empty state", () => {
  it("asks for a tile before one is loaded", async () => {
    serve();
    const app = mount(App);
    await flushPromises();
    expect(app.get(".empty h1").text()).toBe("MapLibre Tile Analyzer");
  });

  it("counts the catalogue on the button that opens the sheet", async () => {
    serve();
    const app = mount(App);
    await flushPromises();
    expect(app.get("button.open").text()).toBe("Browse one of 2 fixtures");
  });

  it("omits the count while the index is still in flight", async () => {
    serve();
    const app = mount(App);
    expect(app.get("button.open").text()).toBe("Browse the fixtures");
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
    expect(app.findAll("dialog .entry .name").map((e) => e.text())).toEqual([
      "0x01/point.mlt",
      "0x02/line.mlt",
    ]);
  });

  it("narrows the sheet to the fixtures the filter names", async () => {
    serve();
    const app = mount(App);
    await flushPromises();
    await app.get("button.open").trigger("click");
    await app.get("dialog .filter").setValue("0x02");
    expect(app.findAll("dialog .entry .name").map((e) => e.text())).toEqual([
      "0x02/line.mlt",
    ]);
  });

  it("finds a fixture from letters spread over its key", async () => {
    serve();
    const app = mount(App);
    await flushPromises();
    await app.get("button.open").trigger("click");
    await app.get("dialog .filter").setValue("ln");
    expect(app.findAll("dialog .entry .name").map((e) => e.text())).toEqual([
      "0x02/line.mlt",
    ]);
  });

  it("says so when the filter matches no fixture", async () => {
    serve();
    const app = mount(App);
    await flushPromises();
    await app.get("button.open").trigger("click");
    await app.get("dialog .filter").setValue("nothing");
    expect(app.get("dialog .none").text()).toBe(
      "No fixture matches that filter.",
    );
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

  it("annotates an added tile", async () => {
    serve();
    const app = mount(App);
    await flushPromises();
    app
      .findComponent(SourcePicker)
      .vm.$emit("file", new File([new Uint8Array(8)], "own.mlt"));
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

  it("drops a region an added tile cannot name", async () => {
    serve();
    history.replaceState(null, "", "/?fixture=0x01/point.mlt&region=1");
    const app = mount(App);
    await flushPromises();
    app
      .findComponent(SourcePicker)
      .vm.$emit("file", new File([new Uint8Array(8)], "own.mlt"));
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
