import { afterEach, describe, expect, it, vi } from "vitest";
import { followScheme, isEmbedded, resolveScheme } from "./theme.ts";

/** A same-origin parent the bridge can read and observe, which jsdom cannot hand over as a window. */
function docsPage(scheme: string | null) {
  const page = { document: document.implementation.createHTMLDocument() };
  if (scheme !== null) page.document.body.dataset.mdColorScheme = scheme;
  return page;
}

function prefersDark(dark: boolean) {
  vi.stubGlobal("matchMedia", () => ({
    matches: dark,
    addEventListener: () => {},
    removeEventListener: () => {},
  }));
}

/** A MutationObserver delivers at the end of the task, not on the write. */
function delivered() {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

afterEach(() => {
  vi.unstubAllGlobals();
  delete document.documentElement.dataset.theme;
});

describe("isEmbedded", () => {
  it("is false when the app owns its window", () => {
    expect(isEmbedded()).toBe(false);
  });

  it("is true once a parent window is above it", () => {
    vi.stubGlobal("parent", docsPage("slate"));
    expect(isEmbedded()).toBe(true);
  });
});

describe("resolveScheme", () => {
  it("reads slate off the docs page as dark", () => {
    prefersDark(false);
    expect(resolveScheme(docsPage("slate"))).toBe("dark");
  });

  it("reads default off the docs page as light", () => {
    prefersDark(true);
    expect(resolveScheme(docsPage("default"))).toBe("light");
  });

  it("takes the OS preference standalone", () => {
    prefersDark(true);
    expect(resolveScheme(window)).toBe("dark");
  });

  it("takes the OS preference when the parent is cross-origin", () => {
    prefersDark(true);
    const blocked = {
      get document(): Document {
        throw new DOMException("cross-origin");
      },
    };
    expect(resolveScheme(blocked)).toBe("dark");
  });

  it("takes the OS preference when the docs page names no palette", () => {
    prefersDark(false);
    expect(resolveScheme(docsPage(null))).toBe("light");
  });
});

describe("followScheme", () => {
  it("paints the docs page's scheme on the document element", () => {
    prefersDark(false);
    followScheme(docsPage("slate"))();
    expect(document.documentElement.dataset.theme).toBe("dark");
  });

  it("repaints when the docs toggle rewrites the attribute", async () => {
    prefersDark(false);
    const page = docsPage("default");
    const unfollow = followScheme(page);
    page.document.body.dataset.mdColorScheme = "slate";
    await delivered();
    unfollow();
    expect(document.documentElement.dataset.theme).toBe("dark");
  });

  it("stops repainting once torn down", async () => {
    prefersDark(false);
    const page = docsPage("default");
    followScheme(page)();
    page.document.body.dataset.mdColorScheme = "slate";
    await delivered();
    expect(document.documentElement.dataset.theme).toBe("light");
  });
});
