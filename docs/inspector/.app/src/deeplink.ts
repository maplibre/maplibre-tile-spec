/** The `fixture`, `url`, `layer` and `region` query parameters, the whole deep-link contract. */

import { tileAddress } from "./fixtures.ts";

export interface DeepLink {
  /** Index key of a synthetic fixture, `<dir>/<name>`. An added tile has none. */
  fixture: string | null;
  /** Address a tile was fetched from, for one that is not in the index. */
  url: string | null;
  /** Index of the top-level layer the tree is filtered to. */
  layer: number | null;
  /** Index of the selected region in the tree as filtered by `layer`. */
  region: number | null;
}

export function readDeepLink(search: string): DeepLink {
  const params = new URLSearchParams(search);
  const raw = params.get("url");
  return {
    fixture: params.get("fixture") || null,
    url: raw === null ? null : tileAddress(raw),
    layer: index(params.get("layer")),
    region: index(params.get("region")),
  };
}

/** The query string for a link, empty when it carries nothing, so a bare app keeps a bare URL. */
export function deepLinkSearch(link: DeepLink): string {
  const params = new URLSearchParams();
  if (link.fixture !== null) params.set("fixture", link.fixture);
  if (link.url !== null) params.set("url", link.url);
  if (link.layer !== null) params.set("layer", String(link.layer));
  if (link.region !== null) params.set("region", String(link.region));
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
