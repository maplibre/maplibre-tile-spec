/** The palette the app paints, which follows the docs page's toggle while embedded in one. */

import { useEventListener, useMutationObserver } from "@vueuse/core";

export type Scheme = "dark" | "light";

/** What the bridge reads of the parent, so a test can hand it a document instead of a window. */
export interface DocsParent {
  document: Document;
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
  const unlisten = useEventListener(
    matchMedia("(prefers-color-scheme: dark)"),
    "change",
    update,
  );
  const { stop } = useMutationObserver(parentBody(parent), update, {
    attributeFilter: ["data-md-color-scheme"],
  });
  return () => {
    unlisten();
    stop();
  };
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
