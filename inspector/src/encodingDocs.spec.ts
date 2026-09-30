import { join } from "node:path";
import { flushPromises, mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { readEncodingDocs } from "../plugins/encoding-docs.ts";
import EncodingDocs from "./EncodingDocs.vue";
import {
  docKeyFor,
  everyDocKey,
  regionAnchors,
  resetEncodingDocs,
  sectionsFor,
} from "./encodingDocs.ts";

const docs = readEncodingDocs(join(import.meta.dirname, "..", "..", "docs"));

describe("the docs the app points at", () => {
  it("resolves every section it can ask for", () => {
    const missing = everyDocKey().filter((key) => docs[key] === undefined);
    expect(missing).toEqual([]);
  });

  /**
   * Every logical and physical id the fixtures' streams report, as `BlobInfo` spells
   * them. Listed here rather than read off the tiles, since the wasm does not load
   * under vitest; a format that gains an encoding belongs on this list, and fails
   * until its section is written.
   */
  const STREAM_IDS = [
    "bool/byte-rle",
    "bool/none",
    "float/alp",
    "float/dict",
    "float/none",
    "int/delta",
    "int/delta-rle",
    "int/none",
    "int/rle",
    "vertex/componentwise-delta",
    "vertex/morton-delta",
    "bit-packed",
    "fastpfor[128le]",
    "fastpfor[256be]",
    "none",
    "varint",
  ];

  it("resolves a section for every encoding a stream reports", () => {
    const unresolved = STREAM_IDS.filter((id) => {
      const key = docKeyFor(id);
      return key === undefined || docs[key] === undefined;
    });
    expect(unresolved).toEqual([]);
  });

  it("reads a section out of the encodings page", () => {
    const [varint] = sectionsFor(["encodings#varint"], docs);
    expect(varint.title).toBe("VarInt");
    expect(varint.html).toContain("7-bit groups");
  });

  /** A link back into the app is noise inside it, and the renderer has no attr_list. */
  it("drops the inspector examples but keeps the prose beside them", () => {
    const [varint] = sectionsFor(["encodings#varint"], docs);
    expect(varint.html).toContain("7-bit groups");
    expect(varint.html).not.toContain("in the inspector");
    expect(varint.html).not.toContain("{target=_blank}");

    const all = Object.values(docs).map((doc) => doc.html);
    expect(all.some((html) => html.includes("7-bit groups"))).toBe(true);
    for (const html of all) {
      expect(html).not.toContain("inspector/app/?fixture=");
      expect(html).not.toContain("{target=_blank}");
    }
  });

  /** The app ships inside the site, so a fork or a preview links into its own pages. */
  it("links a section relative to where the site embeds the app", () => {
    expect(docs["encodings#alp"]?.site).toBe("../../encodings/#alp");
    expect(docs["v2#tessellation"]?.site).toBe(
      "../../specification/v2/#tessellation",
    );
  });

  it("resolves a link against its own page before re-basing it", () => {
    // `../encodings.md` in specification/v2.md is docs/encodings.md, not one above it.
    const html = Object.values(docs)
      .map((doc) => doc.html)
      .join("");
    expect(html).not.toContain('href="../../../');
  });

  it("reads a section out of the v2 spec", () => {
    const [byte] = sectionsFor(["v2#column-type-byte"], docs);
    expect(byte.title).toBe("Column Type Byte");
  });

  it("keeps two pages' identical anchors apart", () => {
    expect(docs["v1#encoding-byte"]).toBeDefined();
    expect(docs["v2#encoding-byte"]).toBeDefined();
    expect(docs["v1#encoding-byte"]?.html).not.toBe(
      docs["v2#encoding-byte"]?.html,
    );
  });

  it("drops a key the pages do not define", () => {
    expect(sectionsFor(["encodings#no-such-thing"], docs)).toEqual([]);
  });
});

/** The v2 point tile, as `mlt hexdump` lays it out: one layer holding one stream. */
const region = (depth: number, label: string, blob: unknown = null) => ({
  offset: 0,
  len: 1,
  depth,
  label,
  value: null,
  bits: [],
  kind: blob === null ? "meta" : "dataBlob",
  container: false,
  blob,
});
const POINT_TILE = [
  region(0, 'layer[0] "layer1"'),
  region(1, "size"),
  region(1, "tag"),
  region(1, "name"),
  region(1, "header"),
  region(1, "feature_count"),
  region(1, "layout"),
  region(1, "geometry"),
  region(2, "vertices"),
  region(3, "encoding"),
  region(3, "num_values"),
  region(3, "byte_length"),
  region(3, "data", {
    streamType: "data[vertex]",
    logical: "vertex/componentwise-delta",
    physical: "varint",
    numValues: 2,
    hint: { kind: "i32" },
  }),
  region(1, "column_count"),
  // biome-ignore lint/suspicious/noExplicitAny: a hand-built tree standing in for the wasm
] as any[];

describe("the encodings a region is shown", () => {
  /** A stream's header sits beside its payload; a layer's framing does not. */
  it("does not give the layer framing a nested stream's encoding", () => {
    expect(regionAnchors(POINT_TILE, 1, "v2")).toEqual(["v2#tile-layout"]);
  });

  it("reads the payload beside a stream's own header field", () => {
    expect(regionAnchors(POINT_TILE, 9, "v2")).toEqual([
      "v2#encoding-byte",
      "encodings#componentwise-delta",
      "encodings#varint",
    ]);
  });
});

const SECTION = {
  title: "Tile Layout",
  html: "<p>A tile is a sequence of layers.</p>",
  site: "../../specification/v2/#tile-layout",
  edit: "https://github.com/x/edit/main/docs/specification/v2.md#L41",
};

beforeEach(() => {
  resetEncodingDocs();
  vi.stubGlobal("fetch", async () => ({
    ok: true,
    json: async () => ({ "v2#tile-layout": SECTION }),
  }));
});

async function panel(editable = false) {
  const view = mount(EncodingDocs, {
    props: { anchors: ["v2#tile-layout"], editable },
  });
  await flushPromises();
  return view;
}

describe("a docs section", () => {
  it("makes its heading a link to the section on the site", async () => {
    const title = (await panel()).get("h4 a.title");
    expect(title.text()).toBe("Tile Layout");
    expect(title.attributes("href")).toBe(SECTION.site);
  });

  /** The site embeds the app in a frame, so a link has to leave it to be readable. */
  it("opens the link outside the frame", async () => {
    expect((await panel()).get("h4 a.title").attributes("target")).toBe(
      "_blank",
    );
  });

  it("offers the edit link only where there is a pointer to reach it", async () => {
    expect((await panel(false)).findAll("a.edit")).toHaveLength(0);
    expect((await panel(true)).get("a.edit").attributes("href")).toBe(
      SECTION.edit,
    );
  });
});
