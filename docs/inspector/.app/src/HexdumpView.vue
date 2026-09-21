<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import type { DecodedBlob, DumpTree } from "./annotate.ts";
import HexMap from "./HexMap.vue";
import {
  byteOwners,
  fadedBytes,
  fitColumns,
  leafStep,
  type ViewState,
} from "./hex.ts";
import RegionDetail from "./RegionDetail.vue";
import RegionTree from "./RegionTree.vue";

const props = defineProps<{
  tree: DumpTree;
  bytes: Uint8Array;
  decode: (regionIndex: number, maxValues: number) => DecodedBlob;
  /** The walker failure that stopped the annotation, or null. */
  error: string | null;
}>();
const view = defineModel<ViewState>("view", { required: true });
const selected = defineModel<number | null>("selected", { required: true });

const root = ref<HTMLElement | null>(null);
const map = ref<InstanceType<typeof HexMap> | null>(null);
const regions = ref<InstanceType<typeof RegionTree> | null>(null);
const hovered = ref<number | null>(null);
const fit = ref(true);

const activeIndex = computed(() => selected.value ?? hovered.value);
const owners = computed(() => byteOwners(props.tree));
const faded = computed(() => fadedBytes(props.tree, view.value));

const status = computed(() => {
  const tree = props.tree;
  const blobs = tree.regions.filter((region) => region.kind === "dataBlob");
  const blobBytes = blobs.reduce((total, region) => total + region.len, 0);
  const owned = tree.regions
    .filter((region) => !region.container)
    .reduce((total, region) => total + region.len, 0);
  const share = Math.round((100 * blobBytes) / Math.max(1, owned));
  return [
    `${tree.bufLen.toLocaleString()} B`,
    `${tree.regions.length.toLocaleString()} regions`,
    `${blobs.length} blobs (${share}% of annotated bytes)`,
    `${Math.ceil(tree.bufLen / view.value.width).toLocaleString()} hex rows`,
  ].join(" · ");
});

/** Set when this component moved the selection, so an outside change still scrolls the map. */
let picked: number | null = null;

function select(index: number, scroll: boolean) {
  picked = index;
  selected.value = index;
  regions.value?.reveal(index);
  if (scroll) map.value?.scrollToRegion(index);
}

function show(index: number) {
  regions.value?.reveal(index);
  map.value?.scrollToRegion(index);
}

watch(selected, (index) => {
  if (index !== null && index !== picked) show(index);
});

/** Whether the focused control does its own thing with an arrow key, like a slider or a select. */
function consumesArrows(element: Element | null): boolean {
  if (
    element instanceof HTMLSelectElement ||
    element instanceof HTMLTextAreaElement
  )
    return true;
  return element instanceof HTMLInputElement && element.type !== "checkbox";
}

/** ↑ / ↓ walk the leaves, on the window so they work before the map is ever clicked. */
function onKey(event: KeyboardEvent) {
  if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
  if (consumesArrows(document.activeElement)) return;
  event.preventDefault();
  const step = event.key === "ArrowDown" ? 1 : -1;
  const from =
    selected.value ??
    hovered.value ??
    (step > 0 ? -1 : props.tree.regions.length);
  const next = leafStep(props.tree.regions, from, step);
  if (next !== null) select(next, true);
}

function refit() {
  if (fit.value && root.value)
    view.value.width = fitColumns(root.value.clientWidth);
}

let observer: ResizeObserver | null = null;

onMounted(() => {
  window.addEventListener("keydown", onKey);
  if (root.value) {
    observer = new ResizeObserver(refit);
    observer.observe(root.value);
  }
  refit();
  if (selected.value !== null) show(selected.value);
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKey);
  observer?.disconnect();
});

watch(fit, refit);
</script>

<template>
  <div ref="root" class="view">
    <p v-if="props.error" class="walk-error" role="alert">
      <strong>walk stopped:</strong> {{ props.error }} — the remaining bytes are
      <code>&lt;unannotated&gt;</code>
    </p>
    <div class="panes">
      <section class="left">
        <HexMap
          ref="map"
          :tree="props.tree"
          :bytes="props.bytes"
          :owners="owners"
          :view="view"
          :active-index="activeIndex"
          @hover="hovered = $event"
          @pick="select($event, false)"
        />
        <footer>
          <label class="fit"
            ><input v-model="fit" type="checkbox">
            fit width</label
          >
          <span>{{ status }}</span>
          <span v-if="faded" class="faded"
            >max blob fades {{ faded.toLocaleString() }} B</span
          >
        </footer>
      </section>
      <aside>
        <RegionDetail
          :tree="props.tree"
          :bytes="props.bytes"
          :index="activeIndex"
          :sticky="selected !== null"
          :view="view"
          :decode="props.decode"
        />
        <RegionTree
          ref="regions"
          :tree="props.tree"
          :active-index="activeIndex"
          @hover="hovered = $event"
          @pick="select($event, true)"
        />
      </aside>
    </div>
  </div>
</template>

<style scoped>
.view {
  display: flex;
  flex-direction: column;
  min-height: 0;
  flex: 1;
}
.walk-error {
  margin: 0;
  padding: 0.35rem 0.75rem;
  background: var(--warn-bg);
  color: var(--warn);
  font-size: 0.76rem;
}
.panes {
  flex: 1;
  display: grid;
  grid-template-columns: 1fr 22rem;
  min-height: 0;
}
.left {
  display: flex;
  flex-direction: column;
  min-height: 0;
}
footer {
  display: flex;
  gap: 1rem;
  padding: 0.3rem 0.75rem;
  border-top: 1px solid var(--line);
  background: var(--panel);
  color: var(--muted);
  font-size: 0.72rem;
}
.fit {
  display: flex;
  gap: 0.25rem;
  align-items: center;
}
.faded {
  margin-left: auto;
  color: var(--bits);
}
aside {
  border-left: 1px solid var(--line);
  background: var(--panel);
  display: flex;
  flex-direction: column;
  min-height: 0;
}
</style>
