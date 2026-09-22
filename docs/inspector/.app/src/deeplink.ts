/** The `fixture`, `layer` and `region` query parameters, which are the whole deep-link contract. */

export interface DeepLink {
  /** Index key of a synthetic fixture, `<dir>/<name>`. An added tile has none. */
  fixture: string | null;
  /** Index of the top-level layer the tree is filtered to. */
  layer: number | null;
  /** Index of the selected region in the tree as filtered by `layer`. */
  region: number | null;
}

export function readDeepLink(search: string): DeepLink {
  const params = new URLSearchParams(search);
  return {
    fixture: params.get("fixture") || null,
    layer: index(params.get("layer")),
    region: index(params.get("region")),
  };
}

/** The query string for a link, empty when it carries nothing, so a bare app keeps a bare URL. */
export function deepLinkSearch(link: DeepLink): string {
  const params = new URLSearchParams();
  if (link.fixture !== null) params.set("fixture", link.fixture);
  if (link.layer !== null) params.set("layer", String(link.layer));
  if (link.region !== null) params.set("region", String(link.region));
  const search = params.toString();
  return search === "" ? "" : `?${search}`;
}

/** Puts the link in the address bar without adding a history entry per hovered region. */
export function writeDeepLink(link: DeepLink): void {
  history.replaceState(
    null,
    "",
    `${location.pathname}${deepLinkSearch(link)}${location.hash}`,
  );
}

function index(raw: string | null): number | null {
  if (raw === null) return null;
  const value = Number(raw);
  return Number.isInteger(value) && value >= 0 ? value : null;
}
