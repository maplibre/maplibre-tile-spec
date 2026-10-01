/** Slices the encodings page into the sections the inspector shows beside a byte. */

import { existsSync, readFileSync } from "node:fs";
import { join, posix } from "node:path";
import MarkdownIt from "markdown-it";
import type { Plugin } from "vite";
import { everyDocKey } from "../src/encodingDocs.ts";

/**
 * The docs root from `inspector/app/`, where the site embeds this app in a frame.
 *
 * Relative rather than the published address, so a fork or a preview deploy links
 * into its own copy of the pages rather than back to maplibre.org.
 */
const DOCS = "../../";
/**
 * Where a link lands when the app is served on its own, as `npm run dev` serves it.
 *
 * Nothing sits above it there, so a relative link would fall through to the app again
 * at a path it cannot load its own data from. The published pages are the only place
 * such a link can usefully go.
 */
const SITE = "https://maplibre.org/maplibre-tile-spec/";
/** `repo_url` and `edit_uri` of mkdocs.yml, which is where these pages are written. */
const EDIT = "https://github.com/maplibre/maplibre-tile-spec/edit/main/docs";

/** One `{#anchor}` section of the page: its heading, and the body under it as HTML. */
export interface EncodingDoc {
  title: string;
  html: string;
  /** The section on the docs site, for a reader who wants it in full. */
  site: string;
  /** The heading's line in its source file, on the repo's editor. */
  edit: string;
}

const HEADING = /^(#{2,6})\s+(.*?)(?:\s*\{#([a-z0-9-]+)\})?\s*$/;
/** `!!! kind "Title"` and its collapsible `???` form, whose body is the block under it. */
const ADMONITION = /^(?:!!!|\?\?\?\+?)\s+(\S+)(?:\s+"([^"]*)")?\s*$/;
/** `=== "Title"`, one tab of a set, whose body is the block indented under it. */
const TAB = /^(\s*)===\s+"([^"]*)"\s*$/;
/** `--8<-- "path"`, a file spliced in, resolved against the same bases mkdocs.yml lists. */
const SNIPPET = /^(\s*)--8<--\s+"([^"]+)"\s*$/;
const SNIPPET_BASES = ["snippets", "../test/synthetic"];
/** Inline HTML the page uses for a badge, which carries no text of its own. */
const EXPERIMENTAL = /<span class="experimental"><\/span>/g;

const md = MarkdownIt({ html: true, linkify: false });

/** The pages a region or a byte field can point into, and where each one lives. */
export const PAGES = [
  { key: "encodings", file: "encodings.md", site: "encodings/" },
  { key: "v2", file: "specification/v2.md", site: "specification/v2/" },
  { key: "v1", file: "specification/v1.md", site: "specification/v1/" },
];

/** How mkdocs' table of contents turns a heading into an id, for one without `{#id}`. */
function slug(title: string): string {
  return title
    .toLowerCase()
    .replace(/[^\w\s-]/g, "")
    .trim()
    .replace(/\s+/g, "-");
}

/**
 * A section is read outside the page it came from, so a link in it has to be rewritten.
 *
 * A markdown link is relative to its own source file, so `../encodings.md` in
 * `specification/v2.md` has to be resolved against that directory before it can be
 * made relative to the app; prefixing it as written would climb past the site root.
 */
function docsHref(
  href: string,
  page: { file: string; site: string },
  root: string,
): string {
  if (/^[a-z]+:/.test(href)) return href;
  if (href.startsWith("#")) return `${root}${page.site}${href}`;
  const [path, hash] = href.split("#");
  const at = posix.normalize(posix.join(posix.dirname(page.file), path));
  const suffix = hash === undefined ? "" : `#${hash}`;
  return `${root}${at.replace(/\.md$/, "/")}${suffix}`;
}

/**
 * Splices in the files the page includes, which is how every diagram reaches this panel.
 *
 * An SVG needs no extension to display - it is HTML, and the renderer passes it through -
 * so inlining the include is all it takes. The lines keep the include's own indentation,
 * leaving them where the tab and admonition passes below expect to find them.
 */
function expandSnippets(lines: string[], docsDir: string): string[] {
  return lines.flatMap((line) => {
    const hit = SNIPPET.exec(line);
    if (!hit) return [line];
    const [, indent, name] = hit;
    for (const base of SNIPPET_BASES) {
      const file = join(docsDir, base, name);
      if (!existsSync(file)) continue;
      const text = readFileSync(file, "utf8").trimEnd();
      return text.split("\n").map((snippet) => `${indent}${snippet}`);
    }
    throw new Error(
      `${name} is included by the docs but is not under ${SNIPPET_BASES}`,
    );
  });
}

/** Tabs are a mkdocs extension; there is no tab strip here, so each one becomes a heading. */
function flattenTabs(lines: string[]): string[] {
  const out: string[] = [];
  for (let i = 0; i < lines.length; i++) {
    const head = TAB.exec(lines[i]);
    if (!head) {
      out.push(lines[i]);
      continue;
    }
    const [, indent, title] = head;
    out.push(`${indent}**${title}**`, "");
    while (i + 1 < lines.length) {
      const body = lines[i + 1];
      if (body.trim() !== "" && !body.startsWith(`${indent}    `)) break;
      i++;
      out.push(
        body.trim() === "" ? "" : indent + body.slice(indent.length + 4),
      );
    }
  }
  return out;
}

/** Admonitions are a mkdocs extension, so they become blockquotes the renderer knows. */
function foldAdmonitions(lines: string[]): string[] {
  const out: string[] = [];
  for (let i = 0; i < lines.length; i++) {
    const head = ADMONITION.exec(lines[i]);
    if (!head) {
      out.push(lines[i]);
      continue;
    }
    const title = head[2] ?? head[1].charAt(0).toUpperCase() + head[1].slice(1);
    out.push(`> **${title}**`, ">");
    while (i + 1 < lines.length && /^(\s{4}|\s*$)/.test(lines[i + 1])) {
      const body = lines[++i];
      out.push(body.trim() === "" ? ">" : `> ${body.slice(4)}`);
    }
    // Without it the next paragraph lazily continues the blockquote.
    out.push("");
  }
  return out;
}

function render(
  lines: string[],
  page: { file: string; site: string },
  root: string,
  docsDir: string,
): string {
  const folded = foldAdmonitions(flattenTabs(expandSnippets(lines, docsDir)));
  const tokens = md.parse(folded.join("\n"), {});
  for (const token of tokens) {
    if (token.type !== "inline" || !token.children) continue;
    for (const child of token.children) {
      if (child.type !== "link_open") continue;
      const href = child.attrGet("href");
      if (typeof href === "string")
        child.attrSet("href", docsHref(href, page, root));
      child.attrSet("target", "_blank");
      child.attrSet("rel", "noreferrer");
    }
  }
  return md.renderer.render(tokens, md.options, {});
}

/** Every section of one page, keyed by its anchor. */
function readPage(
  docsDir: string,
  page: { key: string; file: string; site: string },
  root: string,
): Record<string, EncodingDoc> {
  const text = readFileSync(join(docsDir, page.file), "utf8").replace(
    EXPERIMENTAL,
    "",
  );
  const docs: Record<string, EncodingDoc> = {};
  let open: { anchor: string; title: string; at: number } | null = null;
  let body: string[] = [];
  const close = () => {
    if (open) {
      docs[open.anchor] = {
        title: open.title,
        html: render(body, page, root, docsDir),
        site: `${root}${page.site}#${open.anchor}`,
        edit: `${EDIT}/${page.file}#L${open.at}`,
      };
    }
    body = [];
  };

  for (const [at, line] of text.split("\n").entries()) {
    const head = HEADING.exec(line);
    if (head) {
      close();
      open = { anchor: head[3] ?? slug(head[2]), title: head[2], at: at + 1 };
      continue;
    }
    // A `#` heading has no anchor of its own and ends whatever was open.
    if (/^#\s/.test(line)) {
      close();
      open = null;
      continue;
    }
    // A link back into the inspector is noise inside it, and its `{target=_blank}`
    // would show as the text the renderer has no attribute list to make sense of.
    if (open && !line.includes("inspector/app/?fixture=")) body.push(line);
  }
  close();
  return docs;
}

/** Every page's sections, keyed `<page>#<anchor>` so two pages may share an anchor. */
export function readEncodingDocs(
  docsDir: string,
  root: string = DOCS,
): Record<string, EncodingDoc> {
  const all: Record<string, EncodingDoc> = {};
  for (const page of PAGES) {
    for (const [anchor, doc] of Object.entries(readPage(docsDir, page, root))) {
      all[`${page.key}#${anchor}`] = doc;
    }
  }
  return all;
}

/**
 * Fails the build when a section the app points at is not in the pages.
 *
 * The app's keys and the pages' anchors are edited apart, so a renamed heading or a
 * dropped `{#anchor}` would otherwise ship as a panel that silently stays empty.
 */
function assertEveryKeyResolves(docs: Record<string, EncodingDoc>): void {
  const missing = everyDocKey().filter((key) => docs[key] === undefined);
  if (missing.length === 0) return;
  throw new Error(
    `encodings.json is missing ${missing.length} section(s) the inspector asks for: ` +
      `${missing.join(", ")}. Add the heading, or its {#anchor}, to the docs page.`,
  );
}

/** Markup of a mkdocs extension, which reaching the rendered HTML means nothing handled it. */
const LEFTOVER: [RegExp, string][] = [
  [/\?\?\?\+?\s/, "admonition"],
  [/8&lt;--/, "snippet include"],
  [/===\s*&quot;/, "tabbed block"],
];

/**
 * Fails the build when a page uses an extension this slicer does not understand.
 *
 * The pages are written for mkdocs, which has extensions this renderer has never heard
 * of, and an unhandled one does not fail - it shows the reader its own markup. Catching
 * it here is the difference between a build error and a panel full of `--8<--`.
 */
function assertNoLeftoverMarkup(docs: Record<string, EncodingDoc>): void {
  const bad = Object.entries(docs).flatMap(([key, doc]) =>
    LEFTOVER.filter(([pattern]) => pattern.test(doc.html)).map(
      ([, kind]) => `${key} (${kind})`,
    ),
  );
  if (bad.length === 0) return;
  throw new Error(
    `encodings.json would ship raw mkdocs markup in ${bad.length} section(s): ` +
      `${bad.join(", ")}. Teach inspector/plugins/encoding-docs.ts that extension.`,
  );
}

export function encodingDocs(): Plugin {
  let docs: Record<string, EncodingDoc> = {};
  let isBuild = false;
  const json = () => `${JSON.stringify(docs, null, 2)}\n`;

  return {
    name: "mlt-encoding-docs",

    configResolved(config) {
      isBuild = config.command === "build";
      docs = readEncodingDocs(
        join(config.root, "..", "docs"),
        isBuild ? DOCS : SITE,
      );
      assertEveryKeyResolves(docs);
      assertNoLeftoverMarkup(docs);
    },

    buildStart() {
      if (!isBuild) return;
      this.emitFile({
        type: "asset",
        fileName: "encodings.json",
        source: json(),
      });
    },

    configureServer(server) {
      server.middlewares.use((request, response, next) => {
        if ((request.url ?? "").split("?")[0] !== "/encodings.json") {
          next();
          return;
        }
        response.setHeader("Content-Type", "application/json");
        response.end(json());
      });
    },
  };
}
