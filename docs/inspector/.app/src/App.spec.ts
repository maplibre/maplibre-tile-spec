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
    renderText: () => "",
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
    expect(app.get(".empty h1").text()).toBe("Annotated hexdump");
  });

  it("offers every indexed fixture once the sheet is open", async () => {
    serve();
    const app = mount(App);
    await flushPromises();
    await app.get("button.open").trigger("click");
    expect(app.findAll("dialog h3").map((group) => group.text())).toEqual([
      "0x01 · point",
      "0x02 · line",
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
      "walk stopped: unexpected end of input — the remaining bytes are <unannotated>",
    );
  });
});
