<style>
  /* Dropped on this page only: under the 61rem grid cap the hex map fits a fraction of the columns the viewport allows. */
  .md-main .md-grid { max-width: none; }
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
</style>

<iframe class="inspector-embed" src="app/index.html" title="MLT Tile Inspector" loading="lazy"></iframe>
