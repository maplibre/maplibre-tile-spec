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
import { stubCanvas, stubDialog, tinyTree } from "./testing.ts";

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
    // Cloned: a body reads once, and a case that opens a second tile fetches twice.
    return tile === null
      ? new Response(null, { status: 404, statusText: "Not Found" })
      : tile.clone();
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
  stubCanvas();
});

beforeEach(() => {
  history.replaceState(null, "", "/");
  vi.mocked(annotateTile).mockReturnValue(handle());
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("the corner link", () => {
  /** The docs page frames the app; standing on its own, `parent` is the window itself. */
  function frame() {
    vi.stubGlobal("parent", { postMessage: () => {} });
  }

  it("is offered on the home screen, before any tile is loaded", async () => {
    frame();
    serve();
    const app = mount(App);
    await flushPromises();
    expect(app.find(".empty a.popout").exists()).toBe(true);
  });

  it("stays offered once a tile is loaded", async () => {
    frame();
    serve();
    const app = mount(App);
    await flushPromises();
    await app.getComponent(SourcePicker).vm.$emit("fixture", "0x01/point.mlt");
    await flushPromises();
    expect(app.find("header a.popout").exists()).toBe(true);
  });

  it("carries the view on screen, so the new window opens on the same tile", async () => {
    frame();
    serve();
    const app = mount(App);
    await flushPromises();
    await app.getComponent(SourcePicker).vm.$emit("fixture", "0x01/point.mlt");
    await flushPromises();
    expect(app.get("header a.popout").attributes("href")).toContain(
      "fixture=0x01%2Fpoint.mlt",
    );
  });

  it("is absent where the app is served bare, with no page on either side of it", async () => {
    serve();
    const app = mount(App);
    await flushPromises();
    expect(app.find("a.popout").exists()).toBe(false);
  });

  /** Published, the app sits in an `app/` folder of the page it is framed by. */
  it("points a window of its own back at the page it was popped out of", async () => {
    serve();
    history.replaceState(
      null,
      "",
      "/inspector/app/index.html?fixture=0x01/point.mlt",
    );
    const app = mount(App);
    await flushPromises();
    const link = app.get("header a.popout");
    expect(link.attributes("href")).toBe(
      "/inspector/?fixture=0x01%2Fpoint.mlt",
    );
  });

  it("stays in the same window on the way back, unlike the way out", async () => {
    serve();
    history.replaceState(null, "", "/inspector/app/?fixture=0x01/point.mlt");
    const app = mount(App);
    await flushPromises();
    expect(app.get("header a.popout").attributes("target")).toBeUndefined();
  });
});

describe("the history a tile leaves", () => {
  /** Every case starts on a tile, which is the entry the next move is measured against. */
  async function opened() {
    serve();
    history.replaceState(null, "", "/?fixture=0x01/point.mlt");
    const app = mount(App);
    await flushPromises();
    return { app, push: vi.spyOn(history, "pushState") };
  }

  it("replaces the entry the app was opened on, which is already the tile's own", async () => {
    serve();
    history.replaceState(null, "", "/?fixture=0x01/point.mlt");
    const push = vi.spyOn(history, "pushState");
    mount(App);
    await flushPromises();
    expect(push).not.toHaveBeenCalled();
  });

  it("leaves an entry behind the tile another one replaces", async () => {
    const { app, push } = await opened();
    await app.getComponent(SourcePicker).vm.$emit("fixture", "0x02/line.mlt");
    await flushPromises();
    expect(push).toHaveBeenCalledOnce();
    expect(location.search).toBe("?fixture=0x02%2Fline.mlt");
  });

  it("leaves one behind the tile the home button drops", async () => {
    const { app, push } = await opened();
    await app.get("button.home").trigger("click");
    await flushPromises();
    expect(push).toHaveBeenCalledOnce();
    expect(location.search).toBe("");
  });

  it("keeps a walked region on the entry its tile opened", async () => {
    const { app, push } = await opened();
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown" }));
    await flushPromises();
    expect(location.search).toBe("?fixture=0x01%2Fpoint.mlt&region=1");
    expect(push).not.toHaveBeenCalled();
    app.unmount();
  });

  it("reopens the tile the Back button lands on", async () => {
    const { app } = await opened();
    await app.getComponent(SourcePicker).vm.$emit("fixture", "0x02/line.mlt");
    await flushPromises();
    expect(app.get("button.open").text()).toBe("0x02/line.mlt");

    history.replaceState(null, "", "/?fixture=0x01/point.mlt");
    window.dispatchEvent(new PopStateEvent("popstate"));
    await flushPromises();
    expect(app.get("button.open").text()).toBe("0x01/point.mlt");
    app.unmount();
  });

  /** Back twice in a row, where the first tile is still in flight when the second lands. */
  it("drops a tile that arrives after the move away from it", async () => {
    const pending: ((tile: Response) => void)[] = [];
    vi.stubGlobal(
      "fetch",
      vi.fn(async (url: string) => {
        if (url === "fixtures.json") return Response.json(index);
        if (url === "fixtures/0x01/point.mlt")
          return new Promise<Response>((resolve) => pending.push(resolve));
        return new Response(new Uint8Array(8));
      }),
    );
    history.replaceState(null, "", "/?fixture=0x02/line.mlt");
    const app = mount(App);
    await flushPromises();

    history.replaceState(null, "", "/?fixture=0x01/point.mlt");
    window.dispatchEvent(new PopStateEvent("popstate"));
    history.replaceState(null, "", "/");
    window.dispatchEvent(new PopStateEvent("popstate"));
    await flushPromises();
    expect(app.find(".empty").exists()).toBe(true);

    pending[0]?.(new Response(new Uint8Array(8)));
    await flushPromises();
    expect(app.find(".empty").exists()).toBe(true);
    app.unmount();
  });

  it("returns to the home screen on the way back past the first tile", async () => {
    const { app } = await opened();
    history.replaceState(null, "", "/");
    window.dispatchEvent(new PopStateEvent("popstate"));
    await flushPromises();
    expect(app.get(".empty h1").text()).toBe("MapLibre Tile Analyzer");
    app.unmount();
  });
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

describe("a tile named by its address", () => {
  const ADDRESS = "https://example.org/14/8298/10748.mlt";

  /** The picker asks for an address; the app is what fetches it. */
  function ask(app: ReturnType<typeof mount>, address: string) {
    app.getComponent(SourcePicker).vm.$emit("url", address);
    return flushPromises();
  }

  it("fetches it without this reader's cookies", async () => {
    const fetched = serve();
    const app = mount(App);
    await flushPromises();
    await ask(app, ADDRESS);
    expect(fetched).toHaveBeenCalledWith(ADDRESS, { credentials: "omit" });
    expect(annotateTile).toHaveBeenCalledOnce();
  });

  it("names it on the picker, which has no index key to show", async () => {
    serve();
    const app = mount(App);
    await flushPromises();
    await ask(app, ADDRESS);
    expect(app.get("button.open").text()).toBe(ADDRESS);
  });

  it("carries it in the deep link, so the tile can be linked to", async () => {
    serve();
    const app = mount(App);
    await flushPromises();
    await ask(app, ADDRESS);
    expect(location.search).toBe(
      "?url=https%3A%2F%2Fexample.org%2F14%2F8298%2F10748.mlt",
    );
  });

  it("opens the one a deep link names", async () => {
    const fetched = serve();
    history.replaceState(null, "", `/?url=${encodeURIComponent(ADDRESS)}`);
    const app = mount(App);
    await flushPromises();
    expect(fetched).toHaveBeenCalledWith(ADDRESS, { credentials: "omit" });
    expect(app.find(".empty").exists()).toBe(false);
  });

  it("says what went wrong when the site holding it refuses the request", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async (url: string) => {
        if (url === "fixtures.json") return Response.json(index);
        throw new TypeError("Failed to fetch");
      }),
    );
    const app = mount(App);
    await flushPromises();
    await ask(app, ADDRESS);
    expect(app.get("[role=alert]").text()).toContain(
      "may not allow requests from other sites",
    );
    expect(app.find(".empty").exists()).toBe(true);
  });

  it("leaves an entry behind the tile it replaces, the same as a fixture does", async () => {
    serve();
    history.replaceState(null, "", "/?fixture=0x01/point.mlt");
    const app = mount(App);
    await flushPromises();
    const push = vi.spyOn(history, "pushState");
    await ask(app, ADDRESS);
    expect(push).toHaveBeenCalledOnce();
  });
});

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
