import { flushPromises, mount } from "@vue/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";
import App from "./App.vue";

const index = [
  { name: "a.mlt", directory: "0x01", bytes: 10 },
  { name: "b.mlt", directory: "0x01", bytes: 20 },
  { name: "c.mlt", directory: "0x02", bytes: 5 },
];

const rows = (html: string) => html.match(/<tbody>.*<\/tbody>/s)?.[0] ?? "";

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("App", () => {
  it("groups the index by directory", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(Response.json(index)));
    const app = mount(App);
    await flushPromises();
    expect(rows(app.html()).replace(/\s+/g, " ")).toBe(
      "<tbody> <tr> <td>0x01</td> <td>2</td> <td>30</td> </tr> <tr> <td>0x02</td> <td>1</td> <td>5</td> </tr> </tbody>",
    );
  });

  it("reports an unreadable index", async () => {
    vi.stubGlobal(
      "fetch",
      vi
        .fn()
        .mockResolvedValue(
          new Response(null, { status: 404, statusText: "Not Found" }),
        ),
    );
    const app = mount(App);
    await flushPromises();
    expect(app.get("[role=alert]").text()).toBe(
      "Error: fixtures.json: 404 Not Found",
    );
  });
});
