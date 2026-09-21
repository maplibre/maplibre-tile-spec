<script setup lang="ts">
import {
  computed,
  nextTick,
  onBeforeUnmount,
  onMounted,
  ref,
  shallowRef,
  watch,
} from "vue";
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

const index = ref<FixtureEntry[]>([]);
const view = ref<ViewState>(defaultView());
const tile = shallowRef<AnnotatedTile | null>(null);
const bytes = shallowRef<Uint8Array>(new Uint8Array());
const fixture = ref<string | null>(null);
const failure = ref<string | null>(null);
const selected = ref<number | null>(null);
const dragging = ref(false);

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

async function pickFixture(key: string) {
  failure.value = null;
  try {
    load(await loadFixture(key), key);
  } catch (cause) {
    failure.value = String(cause);
  }
}

async function pickUpload(file: File) {
  failure.value = null;
  try {
    load(new Uint8Array(await file.arrayBuffer()), null);
  } catch (cause) {
    failure.value = String(cause);
  }
}

/** A tile can be dropped anywhere, including onto the hex map of the tile it replaces. */
function onDragOver(event: DragEvent) {
  event.preventDefault();
  dragging.value = true;
}

function onDragLeave(event: DragEvent) {
  if (event.relatedTarget === null) dragging.value = false;
}

function onDrop(event: DragEvent) {
  event.preventDefault();
  dragging.value = false;
  const file = event.dataTransfer?.files?.[0];
  if (file) void pickUpload(file);
}

onMounted(async () => {
  window.addEventListener("dragover", onDragOver);
  window.addEventListener("dragleave", onDragLeave);
  window.addEventListener("drop", onDrop);
  const link = readDeepLink(location.search);
  try {
    index.value = await loadFixtureIndex();
  } catch (cause) {
    failure.value = String(cause);
  }
  if (link.fixture === null) return;
  await pickFixture(link.fixture);
  if (tile.value === null) return;
  view.value.layer = link.layer;
  await nextTick();
  selected.value = link.region;
});

onBeforeUnmount(() => {
  window.removeEventListener("dragover", onDragOver);
  window.removeEventListener("dragleave", onDragLeave);
  window.removeEventListener("drop", onDrop);
});

/** Changing the layer renumbers the tree, so a selection cannot survive it. */
watch(layer, () => {
  selected.value = null;
});

watch([fixture, layer, selected], () => {
  writeDeepLink({
    fixture: fixture.value,
    layer: layer.value,
    region: selected.value,
  });
});
</script>

<template>
  <div class="app" :class="{ dragging }">
    <header>
      <SourcePicker
        :index="index"
        :current="fixture"
        @fixture="pickFixture"
        @upload="pickUpload"
      />
      <RenderControls v-if="tree" v-model="view" :layers="layers" />
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

    <!-- A view with no tile cannot render an empty state, so the shell owns this one. -->
    <section v-else class="empty">
      <h1>Annotated hexdump</h1>
      <p
        >Choose a synthetic fixture above, or drop a <code>.mlt</code> tile
        anywhere on this page.</p
      >
    </section>
  </div>
</template>

<style>
:root {
  color-scheme: dark;
  --radius: 10px;
  /* Inline byte and tree-row highlights, which `--radius` would round into circles. */
  --radius-inline: 2px;
  /* Gutter of every panel, card and bar. */
  --pad: 1.5rem;
  /* Padding of a row inside a list, and the vertical half of a control. */
  --pad-tight: 0.55rem;
  --bg: #111725;
  --panel: #161e30;
  --control: #1b2437;
  --line: #26304a;
  --text: #d3e2ef;
  --muted: #7386a1;
  --dim: #4e5f7a;
  --blob: #96aec4;
  --faded: #3a4761;
  --rule: #3b4c5e;
  --hover: #1c2639;
  --accent: #2e4f70;
  --accent-text: #eaf4ff;
  --accent-rule: #6fa8dc;
  --container: #e8d9a0;
  --value: #7fd6a0;
  --bits: #d9a441;
  --warn: #e5a0a0;
  --warn-bg: #3a1b1b;
}
body {
  margin: 0;
  background: var(--bg);
  color: var(--text);
  font-family: ui-monospace, "SF Mono", "JetBrains Mono", monospace;
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
.failure {
  margin: 0;
  padding: var(--pad-tight) var(--pad);
  background: var(--warn-bg);
  color: var(--warn);
  font-size: 0.78rem;
}
.empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 0.6rem;
  text-align: center;
}
.empty h1 {
  font:
    600 1.1rem / 1.2 system-ui,
    sans-serif;
  margin: 0;
}
.empty p {
  color: var(--muted);
  margin: 0;
}
</style>
