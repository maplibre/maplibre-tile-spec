---
description: Decode a tile in the browser and walk through its bytes
---

<style>
  /* Dropped on this page only: under the 61rem grid cap the hex map fits a fraction of the columns the viewport allows. */
  .md-main .md-grid { max-width: none; }
  /* The page is a frame around the app, so editing its source leads nowhere a reader wants to go. */
  /* Zensical has no per-page switch for the content actions yet: https://github.com/zensical/backlog/issues/120 */
  .md-content__button { display: none; }
  .inspector-embed {
    display: block;
    width: 100%;
    /* The header measures exactly 2.4rem at every width the site scales its root font to. */
    height: calc(100vh - 2.4rem);
    min-height: 32rem;
    border: 0;
    /* The app's own `--radius`, so the bar it paints against the page ends in the same corner. */
    border-radius: 10px;
    background: var(--md-default-bg-color);
  }
  /* The app names itself, so the page heading would only repeat it. */
  .md-content__inner > h1 { display: none; }
  /* With no heading or prose, the frame sits close to the header and sidebar. */
  .md-main__inner { margin-top: 0.4rem; }
  .md-content__inner { padding-top: 0; }
  .md-content__inner::before { display: none; }
  .md-footer { display: none; }
  .md-content__inner,
  [dir="ltr"] .md-sidebar--primary:not([hidden]) ~ .md-content > .md-content__inner { margin-left: 0.4rem; }
  /* Tall enough for the app's floor, the page stops scrolling and the frame takes whatever the title leaves. */
  @media (min-height: 44rem) {
    body { height: 100dvh; }
    .md-container, .md-main, .md-content { display: flex; flex-direction: column; min-height: 0; }
    .md-main__inner, .md-content, .md-content__inner { flex: 1; min-height: 0; }
    .md-main__inner { width: 100%; }
    .md-content__inner { display: flex; flex-direction: column; }
    .inspector-embed { flex: 1; height: auto; min-height: 0; }
  }
</style>

<iframe class="inspector-embed" src="app/index.html" title="MLT Tile Inspector" loading="lazy"></iframe>

<script>
  // Scoped, not top-level: `navigation.instant` re-runs this script in the page's own
  // global, where a second `const` of the same name would throw.
  (() => {
    const embed = document.querySelector(".inspector-embed");

    // A link back from the app's own window carries the tile in this page's query, which
    // only the frame can act on, so it is handed over before the frame settles on a URL.
    if (location.search) embed.src = "app/index.html" + location.search;

    // The app reports its deep link, so this page's address bar names the tile on screen and a reload keeps it.
    addEventListener("message", (event) => {
      if (event.origin !== location.origin || event.source !== embed.contentWindow) return;
      const state = event.data?.mltInspector;
      if (!state) return;
      if (typeof state.search === "string")
        history.replaceState(null, "", location.pathname + state.search + location.hash);
    });
  })();
</script>
