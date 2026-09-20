import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import App from "./App.vue";

describe("App", () => {
  it("renders the title", () => {
    expect(mount(App).get("h1").text()).toBe("MLT Tile Inspector");
  });
});
