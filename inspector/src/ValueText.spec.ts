import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import ValueText from "./ValueText.vue";

function shown(value: unknown, quoted = false) {
  const view = mount(ValueText, { props: { value, quoted } });
  return {
    text: view.get("span").text(),
    system: view.get("span").classes().includes("system"),
    length: view.find(".len").exists() ? view.get(".len").text() : null,
  };
}

describe("a value the decoder handed back", () => {
  it("prints an absent value as a token, which is otherwise nothing at all", () => {
    expect(shown(null)).toEqual({ text: "null", system: true, length: null });
  });

  it("prints an empty string as a token too, and does not measure it", () => {
    expect(shown("")).toEqual({ text: '""', system: true, length: null });
  });

  it("measures a string, which is what tells a padded value from a short one", () => {
    expect(shown("High St")).toEqual({
      text: "High St",
      system: false,
      length: "(7)",
    });
  });

  it("counts a surrogate pair as the one character it prints as", () => {
    expect(shown("a\u{1F5FA}").length).toBe("(2)");
  });

  it("leaves a number unmeasured, whose digits are not a length", () => {
    expect(shown(4096)).toEqual({ text: "4096", system: false, length: null });
  });
});

describe("a value the dump printed", () => {
  it("keeps the quotes, which are what say the leaf holds text", () => {
    expect(shown('"water"', true)).toEqual({
      text: '"water"',
      system: false,
      length: "(5)",
    });
  });

  it("measures what the quotes hold, not the quotes", () => {
    expect(shown('"a\\"b"', true).length).toBe("(3)");
  });

  it("reads an empty name as a token rather than a pair of quotes", () => {
    expect(shown('""', true)).toEqual({
      text: '""',
      system: true,
      length: null,
    });
  });

  it("leaves an unquoted leaf alone, which the dump only prints for a scalar", () => {
    expect(shown("4096", true)).toEqual({
      text: "4096",
      system: false,
      length: null,
    });
  });

  it("falls back to the quotes for an escape JSON does not share", () => {
    expect(shown('"a\\u{1f}b"', true)).toEqual({
      text: '"a\\u{1f}b"',
      system: false,
      length: "(8)",
    });
  });
});
