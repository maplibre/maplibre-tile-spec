/** The `fixture`, `url`, `layer` and `region` query parameters, the whole deep-link contract. */

import { AXES, tileAddress } from "./fixtures.ts";

export interface DeepLink {
  /** Index key of a synthetic fixture, `<dir>/<name>`. An added tile has none. */
  fixture: string | null;
  /** Address a tile was fetched from, for one that is not in the index. */
  url: string | null;
  /** Index of the top-level layer the tree is filtered to. */
  layer: number | null;
  /** Index of the selected region in the tree as filtered by `layer`. */
  region: number | null;
  /**
   * Label of the region to select, for a link written by hand.
   *
   * The docs point at a section of a tile by what it is called rather than by where it
   * falls, since a position shifts whenever the format or the walker gains a field, and
   * would then quietly name the wrong bytes. Read on the way in and resolved to
   * `region`, never written back.
   */
  at: string | null;
  /** Picked filter chips, each `<axis>:<value>`, so a combination can be handed over. */
  filters: string[];
  /** What the picker's filter box holds, which narrows by name and offers chips. */
  query: string;
  /** Whether the geometry panel is drawn. Only the off state is written down. */
  geo: boolean;
  /** Whether the statistics panel is drawn. Only the on state is written down. */
  stats: boolean;
}

const knownAxes = new Set(AXES.map((a) => a.key));

export function readDeepLink(search: string): DeepLink {
  const params = new URLSearchParams(search);
  const raw = params.get("url");
  return {
    fixture: params.get("fixture") || null,
    url: raw === null ? null : tileAddress(raw),
    layer: index(params.get("layer")),
    region: index(params.get("region")),
    at: params.get("at") || null,
    // Repeated rather than joined: a value may hold any punctuation the spec spells it with.
    filters: params.getAll("f").filter((value) => {
      const colon = value.indexOf(":");
      return colon > 0 && knownAxes.has(value.slice(0, colon));
    }),
    query: params.get("q") ?? "",
    // Shown unless the link says otherwise, so a bare URL opens the whole app.
    geo: params.get("geo") !== "0",
    stats: params.get("stats") === "1",
  };
}

/** The query string for a link, empty when it carries nothing, so a bare app keeps a bare URL. */
export function deepLinkSearch(link: DeepLink): string {
  const params = new URLSearchParams();
  if (link.fixture !== null) params.set("fixture", link.fixture);
  if (link.url !== null) params.set("url", link.url);
  if (link.layer !== null) params.set("layer", String(link.layer));
  if (link.region !== null) params.set("region", String(link.region));
  for (const picked of link.filters) params.append("f", picked);
  if (link.query !== "") params.set("q", link.query);
  if (!link.geo) params.set("geo", "0");
  if (link.stats) params.set("stats", "1");
  const search = params.toString();
  return search === "" ? "" : `?${search}`;
}

/**
 * Puts the link in the address bar.
 *
 * A `fresh` link is one that opened another tile, which is a place the Back button should
 * return to; a layer or a region is a move within the tile, and replaces what is there
 * rather than leaving an entry behind every byte the reader walks over.
 */
export function writeDeepLink(link: DeepLink, fresh = false): void {
  const url = `${location.pathname}${deepLinkSearch(link)}${location.hash}`;
  if (fresh) history.pushState(null, "", url);
  else history.replaceState(null, "", url);
}

function index(raw: string | null): number | null {
  if (raw === null) return null;
  const value = Number(raw);
  return Number.isInteger(value) && value >= 0 ? value : null;
}
