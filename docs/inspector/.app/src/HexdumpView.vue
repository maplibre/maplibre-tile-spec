<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import type { DecodedBlob, DumpTree } from "./annotate.ts";
import HexMap from "./HexMap.vue";
import { byteOwners, fitColumns, leafStep, type ViewState } from "./hex.ts";
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

const activeIndex = computed(() => selected.value ?? hovered.value);
const owners = computed(() => byteOwners(props.tree));

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

/** Whether the focused control does its own thing with an arrow key, like the fixture filter or a select. */
function consumesArrows(element: Element | null): boolean {
  return (
    element instanceof HTMLSelectElement ||
    element instanceof HTMLTextAreaElement ||
    element instanceof HTMLInputElement
  );
}

/** ↑ / ↓ walk the leaves, on the window so they work before the map is ever clicked. */
function onKey(event: KeyboardEvent) {
  if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
  if (document.activeElement?.closest("dialog[open]")) return;
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
  if (root.value) view.value.width = fitColumns(root.value.clientWidth);
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
  padding: var(--pad-tight) var(--pad);
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
aside {
  border-left: 1px solid var(--line);
  background: var(--panel);
  display: flex;
  flex-direction: column;
  min-height: 0;
}
</style>
