/** Slices the encodings page into the sections the inspector shows beside a byte. */

import { readFileSync } from "node:fs";
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
/** `!!! kind "Title"`, whose body is the indented block under it. */
const ADMONITION = /^!!!\s+(\S+)(?:\s+"([^"]*)")?\s*$/;
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
      if (body.trim() !== "") out.push(`> ${body.slice(4)}`);
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
): string {
  const tokens = md.parse(foldAdmonitions(lines).join("\n"), {});
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
        html: render(body, page, root),
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
    if (open) body.push(line);
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

export function encodingDocs(): Plugin {
  let docs: Record<string, EncodingDoc> = {};
  let isBuild = false;
  const json = () => `${JSON.stringify(docs, null, 2)}\n`;

  return {
    name: "mlt-encoding-docs",

    configResolved(config) {
      isBuild = config.command === "build";
      docs = readEncodingDocs(
        join(config.root, "..", ".."),
        isBuild ? DOCS : SITE,
      );
      assertEveryKeyResolves(docs);
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
