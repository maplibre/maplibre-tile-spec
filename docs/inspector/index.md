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
  /* Tall enough for the app's floor, the page stops scrolling and the frame takes whatever the title leaves. */
  @media (min-height: 44rem) {
    body { height: 100dvh; }
    .md-footer { display: none; }
    .md-container, .md-main, .md-content { display: flex; flex-direction: column; min-height: 0; }
    .md-main__inner, .md-content, .md-content__inner { flex: 1; min-height: 0; }
    .md-main__inner { width: 100%; }
    .md-content__inner { display: flex; flex-direction: column; }
    .inspector-embed { flex: 1; height: auto; min-height: 0; }
  }
</style>

<iframe class="inspector-embed" src="app/index.html" title="MLT Tile Inspector" loading="lazy"></iframe>
