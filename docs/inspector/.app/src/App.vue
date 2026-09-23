<script setup lang="ts">
import { useDropZone } from "@vueuse/core";
import { computed, nextTick, onMounted, ref, shallowRef, watch } from "vue";
import {
  type AnnotatedTile,
  annotateTile,
  type DecodedBlob,
} from "./annotate.ts";
import { readDeepLink, writeDeepLink } from "./deeplink.ts";
import {
  type FixtureEntry,
  loadFixture,
  loadFixtureIndex,
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
const bytes = shallowRef<Uint8Array>(new Uint8Array());
const fixture = ref<string | null>(null);
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

function load(raw: Uint8Array, key: string | null) {
  tile.value?.free();
  tile.value = annotateTile(raw);
  bytes.value = raw;
  fixture.value = key;
  selected.value = null;
  view.value.layer = null;
}

/** Drops the tile so the empty state takes over, which is the app's home screen. */
function goHome() {
  tile.value?.free();
  tile.value = null;
  bytes.value = new Uint8Array();
  fixture.value = null;
  failure.value = null;
  selected.value = null;
  view.value.layer = null;
}

async function pickFixture(key: string) {
  failure.value = null;
  try {
    load(await loadFixture(key), key);
  } catch (cause) {
    failure.value = String(cause);
  }
}

async function pickFile(file: File) {
  failure.value = null;
  try {
    load(new Uint8Array(await file.arrayBuffer()), null);
  } catch (cause) {
    failure.value = String(cause);
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

onMounted(async () => {
  const initial = readDeepLink(location.search);
  try {
    index.value = await loadFixtureIndex();
  } catch (cause) {
    failure.value = String(cause);
  }
  if (initial.fixture === null) return;
  await pickFixture(initial.fixture);
  if (tile.value === null) return;
  view.value.layer = initial.layer;
  await nextTick();
  selected.value = initial.region;
});

/** Changing the layer renumbers the tree, so a selection cannot survive it. */
watch(layer, () => {
  selected.value = null;
});

const link = computed(() => ({
  fixture: fixture.value,
  layer: layer.value,
  region: selected.value,
}));

watch(link, (current) => {
  writeDeepLink(current);
});
</script>

<template>
  <div ref="root" class="app" :class="{ dragging }">
    <header v-if="tree">
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
        :current="fixture"
        @fixture="pickFixture"
        @file="pickFile"
      />
      <RenderControls v-model="view" :layers="layers" />
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
    />
    <section v-else class="empty">
      <h1>Inspect MLT internals</h1>
      <SourcePicker
        hero
        :index="index"
        :current="fixture"
        @fixture="pickFixture"
        @file="pickFile"
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
.home {
  background: var(--control);
  color: var(--text);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  /* U+2302 sits small on its em, so it needs more than the text size around it. */
  font-size: 1.2rem;
  line-height: 1;
  padding: 0.4rem 0.7rem;
  cursor: pointer;
}
.home:hover {
  background: var(--hover);
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
