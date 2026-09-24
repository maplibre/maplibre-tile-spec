<script setup lang="ts">
import { useDropZone, useEventListener } from "@vueuse/core";
import type { FeatureCollection } from "geojson";
import { computed, nextTick, onMounted, ref, shallowRef, watch } from "vue";
import {
  type AnnotatedTile,
  annotateTile,
  type DecodedBlob,
  tileGeoJson,
} from "./annotate.ts";
import {
  type DeepLink,
  deepLinkSearch,
  readDeepLink,
  writeDeepLink,
} from "./deeplink.ts";
import {
  type FixtureEntry,
  loadFixture,
  loadFixtureIndex,
  loadTile,
} from "./fixtures.ts";
import HexdumpView from "./HexdumpView.vue";
import {
  defaultView,
  layerLabels,
  type ViewState,
  wholeIndices,
} from "./hex.ts";
import RenderControls from "./RenderControls.vue";
import SourcePicker from "./SourcePicker.vue";
import { followScheme } from "./theme.ts";

const root = ref<HTMLElement | null>(null);
const index = ref<FixtureEntry[]>([]);
const view = ref<ViewState>(defaultView());
const tile = shallowRef<AnnotatedTile | null>(null);
const decoded = shallowRef<FeatureCollection | null>(null);
const bytes = shallowRef<Uint8Array>(new Uint8Array());
/** Index key of the loaded tile, or null when it came from anywhere but the index. */
const fixture = ref<string | null>(null);
/** Address the loaded tile was fetched from, or null when it was not fetched from one. */
const href = ref<string | null>(null);
const failure = ref<string | null>(null);
const selected = ref<number | null>(null);

const whole = computed(() => tile.value?.tree() ?? null);
const layers = computed(() =>
  whole.value === null ? [] : layerLabels(whole.value),
);

/** The layer knob, ignored while it names a layer this tile does not have. */
const layer = computed(() =>
  view.value.layer !== null && view.value.layer < layers.value.length
    ? view.value.layer
    : null,
);

const tree = computed(() => {
  if (whole.value === null) return null;
  return layer.value === null
    ? whole.value
    : (tile.value?.tree(layer.value) ?? whole.value);
});

/** `decodeBlob` counts regions of the whole tile, so a filtered tree has to map back. */
const wholeIndex = computed(() =>
  layer.value === null || whole.value === null || tree.value === null
    ? null
    : wholeIndices(whole.value.regions, tree.value.regions),
);

function decode(regionIndex: number, maxValues: number): DecodedBlob {
  const handle = tile.value;
  const at = wholeIndex.value?.[regionIndex] ?? regionIndex;
  if (handle === null || at < 0)
    return { kind: "error", message: "no such region" };
  try {
    return handle.decodeBlob(at, maxValues);
  } catch (cause) {
    return { kind: "error", message: String(cause) };
  }
}

/** Only this writes where the tile came from, so the two can never name different ones. */
function load(raw: Uint8Array, from: { fixture?: string; href?: string }) {
  tile.value?.free();
  tile.value = annotateTile(raw);
  // The annotated walk survives a tile the decoder chokes on, so the panel goes quiet
  // rather than taking the whole view down with it.
  try {
    decoded.value = tileGeoJson(raw);
  } catch {
    decoded.value = null;
  }
  bytes.value = raw;
  fixture.value = from.fixture ?? null;
  href.value = from.href ?? null;
  selected.value = null;
  view.value.layer = null;
}

/**
 * Counts the moves between views, so a tile that arrives after the next move began is
 * dropped rather than shown. Back pressed twice starts a restore while the one before it
 * is still fetching, and the slower of the two would otherwise land last and leave the app
 * on a tile the address bar no longer names.
 */
let moves = 0;

/** Begins a move and hands back its number, which every step after an await re-checks. */
function move(): number {
  moves += 1;
  return moves;
}

/** Whether `at` is still the move in hand, or a later one has taken the view over. */
function current(at: number): boolean {
  return at === moves;
}

/** Drops the tile so the empty state takes over, which is the app's home screen. */
function goHome() {
  move();
  tile.value?.free();
  tile.value = null;
  decoded.value = null;
  bytes.value = new Uint8Array();
  fixture.value = null;
  href.value = null;
  failure.value = null;
  selected.value = null;
  view.value.layer = null;
}

/** Fetches `key` and shows it, as the move `at`, which a later move cancels. */
async function open(key: string, at: number) {
  failure.value = null;
  try {
    const raw = await loadFixture(key);
    if (current(at)) load(raw, { fixture: key });
  } catch (cause) {
    if (current(at)) failure.value = String(cause);
  }
}

function pickFixture(key: string): Promise<void> {
  return open(key, move());
}

/** The same, for a tile named by its address rather than by an index key. */
async function fetchTile(address: string, at: number) {
  failure.value = null;
  try {
    const raw = await loadTile(address);
    if (current(at)) load(raw, { href: address });
  } catch (cause) {
    if (current(at)) failure.value = fetchFailure(address, cause);
  }
}

function pickUrl(address: string): Promise<void> {
  return fetchTile(address, move());
}

/** A blocked cross-origin fetch rejects with "Failed to fetch" and names no cause. */
function fetchFailure(address: string, cause: unknown): string {
  return cause instanceof TypeError
    ? `${address} could not be fetched - the server it is on may not allow requests from other sites`
    : String(cause);
}

async function pickFile(file: File) {
  const at = move();
  failure.value = null;
  try {
    const raw = new Uint8Array(await file.arrayBuffer());
    if (current(at)) load(raw, {});
  } catch (cause) {
    if (current(at)) failure.value = String(cause);
  }
}

/** The root fills the viewport, so a tile can be dropped anywhere, including onto the hex map. */
const { isOverDropZone: dragging } = useDropZone(root, {
  onDrop: (files) => {
    const file = files?.[0];
    if (file) void pickFile(file);
  },
});

followScheme();

/** What names the tile in a link, whichever of the two ways it was given. */
function tileOf(link: DeepLink): string | null {
  return link.fixture ?? link.url;
}

/** Tile the address bar is already on, so restoring a link does not double its entry. */
let shown = tileOf(readDeepLink(location.search));

/** Puts the app on the view a link names, reloading the tile only when it is another one. */
async function restore(target: DeepLink) {
  if (tileOf(target) === null) {
    goHome();
    return;
  }
  const at = move();
  if (target.fixture !== null && target.fixture !== fixture.value)
    await open(target.fixture, at);
  else if (target.url !== null && target.url !== href.value)
    await fetchTile(target.url, at);
  if (!current(at) || tile.value === null) return;
  view.value.layer = target.layer;
  await nextTick();
  if (current(at)) selected.value = target.region;
}

onMounted(async () => {
  const initial = readDeepLink(location.search);
  try {
    index.value = await loadFixtureIndex();
  } catch (cause) {
    failure.value = String(cause);
  }
  // Not `restore`: with no tile to open there is nothing to go home from, and the reset
  // would take an index failure off the screen with it.
  if (tileOf(initial) !== null) await restore(initial);
  booted.value = true;
});

/** The Back button walks the tiles this app has shown before it leaves the app. */
useEventListener(window, "popstate", () => {
  const target = readDeepLink(location.search);
  shown = tileOf(target);
  void restore(target);
});

/** Changing the layer renumbers the tree, so a selection cannot survive it. */
watch(layer, () => {
  selected.value = null;
});

const link = computed<DeepLink>(() => ({
  fixture: fixture.value,
  url: href.value,
  layer: layer.value,
  region: selected.value,
}));

watch(link, (moved) => {
  const fresh = tileOf(moved) !== shown;
  shown = tileOf(moved);
  writeDeepLink(moved, fresh);
});

/** Only the docs page frames the app; a window of its own has nothing to pop out of. */
const framed = window.parent !== window;

/**
 * The docs page embeds the app from an `app/` folder beside itself, so a window of its own
 * finds the page it came from by dropping that folder. Served bare in development there is
 * no such page, and the corner stays empty.
 */
const EMBED = /app\/(index\.html)?$/;
const embedded = !framed && EMBED.test(location.pathname);

/** The way out of the frame, whichever side it is on. */
interface Corner {
  href: string;
  /** A new window for the way out; the way back replaces the one it is in. */
  target?: string;
  glyph: string;
  label: string;
}

/** Carries the view on screen, so the page or window it opens lands on this same tile. */
const corner = computed<Corner>(() => {
  const search = deepLinkSearch(link.value);
  return framed
    ? {
        href: `${location.pathname}${search}`,
        target: "_blank",
        glyph: "\u2197",
        label: "Open in a new window",
      }
    : {
        href: `${location.pathname.replace(EMBED, "")}${search}`,
        glyph: "\u2199",
        label: "Back to the documentation page",
      };
});

/** Set once the link the app opened on has been applied, so nothing half-loaded is reported. */
const booted = ref(false);

/** Tells the docs page framing us whether the app is on its home screen, so the page can
 * drop its own heading and hand the whole viewport to a loaded tile. The deep link rides
 * along, so the page's own address bar names the tile on screen rather than the one it was
 * opened on, and a reload or a copied URL keeps it. */
watch(
  [booted, () => tree.value !== null, link],
  ([ready, loaded]) => {
    if (!framed || !ready) return;
    window.parent.postMessage(
      { mltInspector: { loaded, search: deepLinkSearch(link.value) } },
      location.origin,
    );
  },
  { immediate: true },
);
</script>

<template>
  <div ref="root" class="app" :class="{ dragging }">
    <header v-if="tree" :class="{ corner: framed || embedded }">
      <button
        type="button"
        class="home"
        title="Home"
        aria-label="Home"
        @click="goHome"
      >
        &#x2302;
      </button>
      <SourcePicker
        :index="index"
        :current="fixture ?? href"
        @fixture="pickFixture"
        @file="pickFile"
        @url="pickUrl"
      />
      <RenderControls v-model="view" :layers="layers" />
      <a
        v-if="framed || embedded"
        class="popout"
        :href="corner.href"
        :target="corner.target"
        rel="noopener"
        :title="corner.label"
        :aria-label="corner.label"
      >
        {{ corner.glyph }}
      </a>
    </header>
    <p v-if="failure" class="failure" role="alert">{{ failure }}</p>
    <HexdumpView
      v-if="tree"
      v-model:view="view"
      v-model:selected="selected"
      :tree="tree"
      :bytes="bytes"
      :decode="decode"
      :error="tile?.error ?? null"
      :tile="decoded"
    />
    <section v-else class="empty">
      <a
        v-if="framed || embedded"
        class="popout"
        :href="corner.href"
        :target="corner.target"
        rel="noopener"
        :title="corner.label"
        :aria-label="corner.label"
      >
        {{ corner.glyph }}
      </a>
      <h1>MapLibre Tile Analyzer</h1>
      <SourcePicker
        hero
        :index="index"
        :current="fixture ?? href"
        @fixture="pickFixture"
        @file="pickFile"
        @url="pickUrl"
      />
    </section>
  </div>
</template>

<style>
/* `data-theme` is written by the blocking read in index.html and kept by src/theme.ts. */
:root {
  color-scheme: light;
  --radius: 10px;
  /* The docs site's two families, loaded from the same Google Fonts URL its pages use. */
  --font: "Inter", system-ui, -apple-system, "Segoe UI", Helvetica, sans-serif;
  --mono: "JetBrains Mono", ui-monospace, "SF Mono", Menlo, monospace;
  /* Inline byte and tree-row highlights, which `--radius` would round into circles. */
  --radius-inline: 5px;
  /* Gutter of every panel, card and bar. */
  --pad: 1.5rem;
  /* Padding of a row inside a list, and the vertical half of a control. */
  --pad-tight: 0.55rem;
  --bg: #ffffff;
  --panel: #f2f5fa;
  --control: #ffffff;
  --line: #ccd6e5;
  --text: #141c2c;
  --muted: #5a6b86;
  --dim: #76849c;
  --blob: #4a5d78;
  --faded: #a9b6c9;
  --rule: #b0bdd0;
  --hover: #e9eff8;
  --accent: #bfdcfb;
  --accent-text: #0d2540;
  --accent-rule: #285daa;
  /* --md-code-hl-keyword / -string / -constant, in the default scheme. */
  --container: #3f6ec6;
  --value: #1c7d4d;
  --bits: #6e59d9;
  /* The block palette: those three hues plus three more, cycled over the containers. */
  --hue-0: #3f6ec6;
  --hue-1: #1c7d4d;
  --hue-2: #6e59d9;
  --hue-3: #b26a12;
  --hue-4: #12808f;
  --hue-5: #b2437f;
  /* How much of a hue a block tint carries, low enough to stay under the text on top of it. */
  --tint: 15%;
  --warn: #d52a2a;
  --warn-bg: #fbeaea;
  --backdrop: #0f172a59;
}
/* The site's own slate tokens: three surfaces, one foreground ramp, one highlight. */
:root[data-theme="dark"] {
  color-scheme: dark;
  /* --color-backdrop / --color-background / --color-background-subtle, the last of which is also the hover. */
  --bg: #0b0c0f;
  --panel: #16171a;
  --control: #212226;
  --hover: #212226;
  /* --md-default-fg-color flattened onto the backdrop, at .12/.20/.28/.40/.56/.82. */
  --line: #252629;
  --faded: #36373b;
  --rule: #47484c;
  --dim: #616266;
  --muted: #838589;
  --text: #bbbdc2;
  /* --md-code-fg-color's hue and saturation, dropped to sit under --text. */
  --blob: #8b94b1;
  /* --md-typeset-mark-color flattened, which is what the site highlights a run with. */
  --accent: #1c3157;
  --accent-text: #ffffff;
  --accent-rule: #568ad6;
  /* The same three --md-code-hl-* roles, in slate's values. */
  --container: #6791e0;
  --value: #2fb170;
  --bits: #9383e2;
  --hue-0: #6791e0;
  --hue-1: #2fb170;
  --hue-2: #9383e2;
  --hue-3: #d0913a;
  --hue-4: #3fb0c0;
  --hue-5: #dd74ad;
  --tint: 18%;
  --warn: #e6695b;
  --warn-bg: #2c1a1a;
  --backdrop: #0b0c0fcc;
}
body {
  margin: 0;
  background: var(--bg);
  color: var(--text);
  font-family: var(--font);
}
.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
}
</style>

<style scoped>
.app {
  display: flex;
  flex-direction: column;
  height: 100vh;
}
.app.dragging {
  outline: 2px dashed var(--value);
  outline-offset: -2px;
}
header {
  display: flex;
  gap: var(--pad);
  align-items: center;
  flex-wrap: wrap;
  padding: 0.9rem var(--pad);
  border-bottom: 1px solid var(--line);
  background: var(--panel);
}
/* Both children wrap internally, so they must not be shrunk below their content. */
header > * {
  flex: 0 0 auto;
  min-width: 0;
}
.home,
.popout {
  background: var(--control);
  color: var(--text);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  /* U+2302 sits small on its em, so it needs more than the text size around it. */
  font-size: 1.2rem;
  line-height: 1;
  padding: 0.4rem 0.7rem;
  cursor: pointer;
  text-decoration: none;
}
.home:hover,
.popout:hover {
  background: var(--hover);
}
/* The same corner in both states. Out of the flow, or a header wide enough to wrap
   would strand it alone on a second row; the padding keeps the controls from under it. */
header.corner {
  position: relative;
  padding-right: calc(var(--pad) + 2.6rem);
}
header .popout {
  position: absolute;
  top: 0.9rem;
  right: var(--pad);
}
.empty .popout {
  position: absolute;
  top: var(--pad);
  right: var(--pad);
}
.failure {
  margin: 0;
  padding: var(--pad-tight) var(--pad);
  background: var(--warn-bg);
  color: var(--warn);
  font-size: 0.78rem;
}
/* No tile is loaded, so the whole page is the picker rather than a bar above one. */
.empty {
  position: relative;
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 0.8rem;
  padding: var(--pad);
  overflow: auto;
  text-align: center;
}
.empty h1 {
  font: 600 1.5rem / 1.2 var(--font);
  margin: 0;
}
</style>
