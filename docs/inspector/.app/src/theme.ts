/** The palette the app paints, which follows the docs page's toggle while embedded in one. */

export type Scheme = "dark" | "light";

/** What the bridge reads of the parent, so a test can hand it a document instead of a window. */
export interface DocsParent {
  document: Document;
}

/** True while the app runs inside the docs page's iframe. */
export function isEmbedded(): boolean {
  return window.parent !== window;
}

/** The docs page's scheme, or the reader's OS preference when there is no readable parent. */
export function resolveScheme(
  parent: DocsParent | Window = window.parent,
): Scheme {
  return (
    parentScheme(parent) ??
    (matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light")
  );
}

/** The same attribute the blocking read in `index.html` writes before the first paint. */
function applyScheme(scheme: Scheme): void {
  document.documentElement.dataset.theme = scheme;
}

/** Paints the current scheme and follows both the docs toggle and the OS, returning the teardown. */
export function followScheme(
  parent: DocsParent | Window = window.parent,
): () => void {
  const update = () => applyScheme(resolveScheme(parent));
  update();
  const media = matchMedia("(prefers-color-scheme: dark)");
  media.addEventListener("change", update);
  const observer = observeParent(parent, update);
  return () => {
    media.removeEventListener("change", update);
    observer?.disconnect();
  };
}

function observeParent(
  parent: DocsParent | Window,
  update: () => void,
): MutationObserver | null {
  const body = parentBody(parent);
  if (body === null) return null;
  const observer = new MutationObserver(update);
  observer.observe(body, { attributeFilter: ["data-md-color-scheme"] });
  return observer;
}

/** The docs palette resolves its auto state before writing the attribute, so this is never auto. */
function parentScheme(parent: DocsParent | Window): Scheme | null {
  const attr = parentBody(parent)?.dataset.mdColorScheme;
  if (attr === undefined) return null;
  return attr === "slate" ? "dark" : "light";
}

/** A cross-origin parent throws on `document`, which is what falls the app back to the OS. */
function parentBody(parent: DocsParent | Window): HTMLElement | null {
  if (parent === window) return null;
  try {
    return parent.document.body ?? null;
  } catch {
    return null;
  }
}
