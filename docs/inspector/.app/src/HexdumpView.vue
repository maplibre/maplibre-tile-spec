<script setup lang="ts">
import { useEventListener, useResizeObserver } from "@vueuse/core";
import type { FeatureCollection } from "geojson";
import { computed, onMounted, ref, watch } from "vue";
import type { DecodedBlob, DumpTree } from "./annotate.ts";
import GeometryView from "./GeometryView.vue";
import { layersOf } from "./geometry.ts";
import HexMap from "./HexMap.vue";
import HexTip from "./HexTip.vue";
import {
  ancestors,
  byteOwners,
  fitColumns,
  leafStep,
  type Pointer,
  regionBands,
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
  /** The decoded tile the geometry panel draws, absent when it could not be decoded. */
  tile?: FeatureCollection | null;
}>();
const view = defineModel<ViewState>("view", { required: true });
const selected = defineModel<number | null>("selected", { required: true });

const map = ref<InstanceType<typeof HexMap> | null>(null);
const regions = ref<InstanceType<typeof RegionTree> | null>(null);
const hovered = ref<number | null>(null);
/** Null while the hover came from the tree, which sits beside the pane the tip would repeat. */
const pointer = ref<Pointer | null>(null);

const panes = ref<HTMLElement | null>(null);
const left = ref<HTMLElement | null>(null);
const side = ref<HTMLElement | null>(null);
/** Width of the right column in px, and the detail pane's share of it as a percentage. */
const sideWidth = ref(352);
const detailShare = ref(30);
/** The geometry panel's share of the left column, as a percentage. */
const geoShare = ref(38);

const SIDE_MIN = 220;
/** Leaves the map enough for fitColumns' eight-column floor, so a drag cannot clip the bytes. */
const MAP_MIN = 330;
const SHARE_MIN = 10;
const SHARE_MAX = 85;

const clamp = (v: number, lo: number, hi: number) =>
  Math.min(Math.max(v, lo), hi);

function sideMax() {
  const total = panes.value?.clientWidth ?? 0;
  return Math.max(SIDE_MIN, total - MAP_MIN);
}

/** Drags the column gutter: the aside is on the right, so leftward widens it. */
function dragSide(event: PointerEvent) {
  const startX = event.clientX;
  const startWidth = sideWidth.value;
  track(event, (move) => {
    sideWidth.value = clamp(
      startWidth - (move.clientX - startX),
      SIDE_MIN,
      sideMax(),
    );
  });
}

/** Drags the row gutter, which splits the aside between the detail and the tree. */
function dragDetail(event: PointerEvent) {
  track(event, (move) => {
    const box = side.value?.getBoundingClientRect();
    if (!box || box.height === 0) return;
    detailShare.value = clamp(
      ((move.clientY - box.top) / box.height) * 100,
      SHARE_MIN,
      SHARE_MAX,
    );
  });
}

/** Drags the row gutter over the map, which the geometry panel grows upwards into. */
function dragGeo(event: PointerEvent) {
  track(event, (move) => {
    const box = left.value?.getBoundingClientRect();
    if (!box || box.height === 0) return;
    geoShare.value = clamp(
      ((box.bottom - move.clientY) / box.height) * 100,
      SHARE_MIN,
      SHARE_MAX,
    );
  });
}

/** Pointer capture keeps the drag alive over the map's canvas and past the window edge. */
function track(event: PointerEvent, onMove: (move: PointerEvent) => void) {
  const handle = event.currentTarget as HTMLElement;
  handle.setPointerCapture(event.pointerId);
  const move = (at: PointerEvent) => onMove(at);
  const stop = () => {
    handle.removeEventListener("pointermove", move);
    handle.removeEventListener("pointerup", stop);
    handle.removeEventListener("pointercancel", stop);
  };
  handle.addEventListener("pointermove", move);
  handle.addEventListener("pointerup", stop);
  handle.addEventListener("pointercancel", stop);
}

/** Arrow keys move a gutter too, which is the only way to reach one without a pointer.
 * Written out rather than using `@keydown.left`, whose modifier guards a mouse button. */
function onGutterKey(event: KeyboardEvent, which: "side" | "detail" | "geo") {
  const steps: Record<string, number> =
    which === "side"
      ? { ArrowLeft: 16, ArrowRight: -16 }
      : { ArrowUp: -4, ArrowDown: 4 };
  const by = steps[event.key];
  if (by === undefined) return;
  event.preventDefault();
  // onKey would otherwise walk the region list out from under the gutter.
  event.stopPropagation();
  if (which === "side")
    sideWidth.value = clamp(sideWidth.value + by, SIDE_MIN, sideMax());
  else if (which === "detail")
    detailShare.value = clamp(detailShare.value + by, SHARE_MIN, SHARE_MAX);
  // The panel is below its gutter, so up has to grow it rather than shrink it.
  else geoShare.value = clamp(geoShare.value - by, SHARE_MIN, SHARE_MAX);
}

const activeIndex = computed(() => selected.value ?? hovered.value);

/** Walking every feature is not something a hover can afford, so the names are kept. */
const layerNames = computed(() => (props.tile ? layersOf(props.tile) : []));

/**
 * Layer the geometry panel singles out, which only ever comes from the pointer.
 *
 * Nothing else narrows it: a selection stays put while the eye moves on, and the layer
 * knob is about which bytes to read, so neither should quietly hide the rest of the tile.
 * Sweeping the tree or the map walks the layers, and leaving it shows the whole tile again.
 */
const scopeLayer = computed(() => {
  const names = layerNames.value;
  const index = hovered.value;
  if (names.length === 0 || index === null) return null;
  const regions = props.tree.regions;
  const top = ancestors(regions, index)[0] ?? index;
  const at = /^layer\[(\d+)\]$/.exec(regions[top].label);
  return at === null ? null : (names[Number(at[1])] ?? null);
});
const owners = computed(() => byteOwners(props.tree));
/** Shared by the map and the tree, so a row and its bytes take the same tint from one walk. */
const bands = computed(() =>
  view.value.colorful ? regionBands(props.tree) : null,
);

/** Set when this component moved the selection, so an outside change still scrolls the map. */
let picked: number | null = null;

function onMapHover(index: number | null, at: Pointer | null) {
  hovered.value = index;
  pointer.value = at;
}

function onTreeHover(index: number | null) {
  hovered.value = index;
  pointer.value = null;
}

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

/** Both axes walk the same one-dimensional list of leaves. */
const ARROW_STEPS = new Map([
  ["ArrowDown", 1],
  ["ArrowRight", 1],
  ["ArrowUp", -1],
  ["ArrowLeft", -1],
]);

/** The arrows walk the leaves, on the window so they work before the map is ever clicked. */
function onKey(event: KeyboardEvent) {
  const step = ARROW_STEPS.get(event.key);
  if (step === undefined) return;
  if (document.activeElement?.closest("dialog[open]")) return;
  if (consumesArrows(document.activeElement)) return;
  event.preventDefault();
  const from =
    selected.value ??
    hovered.value ??
    (step > 0 ? -1 : props.tree.regions.length);
  const next = leafStep(props.tree.regions, from, step);
  if (next !== null) select(next, true);
}

function refit() {
  // The left pane, not the whole view: the sidebar's width is not the map's to use.
  if (left.value) view.value.width = fitColumns(left.value.clientWidth);
}

useEventListener(window, "keydown", onKey);
useResizeObserver(left, refit);

onMounted(() => {
  refit();
  if (selected.value !== null) show(selected.value);
});
</script>

<template>
  <div class="view">
    <p v-if="props.error" class="walk-error" role="alert">
      <strong>walk stopped:</strong> {{ props.error }} - the remaining bytes are
      <code>&lt;unannotated&gt;</code>
    </p>
    <div ref="panes" class="panes" :style="{ '--side': `${sideWidth}px` }">
      <section ref="left" class="left">
        <HexMap
          ref="map"
          :tree="props.tree"
          :bytes="props.bytes"
          :owners="owners"
          :bands="bands"
          :view="view"
          :active-index="activeIndex"
          @hover="onMapHover"
          @pick="select($event, false)"
        />
        <hr
          class="gutter row"
          aria-orientation="horizontal"
          aria-label="Resize the geometry panel"
          :aria-valuenow="Math.round(geoShare)"
          :aria-valuemin="SHARE_MIN"
          :aria-valuemax="SHARE_MAX"
          tabindex="0"
          @pointerdown.prevent="dragGeo"
          @keydown="onGutterKey($event, 'geo')"
        >
        <GeometryView
          :style="{ flexBasis: `${geoShare}%` }"
          :tile="props.tile ?? null"
          :layer="scopeLayer"
        />
      </section>
      <hr
        class="gutter col"
        aria-orientation="vertical"
        aria-label="Resize the side panel"
        :aria-valuenow="Math.round(sideWidth)"
        :aria-valuemin="SIDE_MIN"
        :aria-valuemax="Math.round(sideMax())"
        tabindex="0"
        @pointerdown.prevent="dragSide"
        @keydown="onGutterKey($event, 'side')"
      >
      <aside ref="side">
        <RegionDetail
          :style="{ flexBasis: `${detailShare}%` }"
          :tree="props.tree"
          :bytes="props.bytes"
          :index="activeIndex"
          :sticky="selected !== null"
          :decode="props.decode"
        />
        <hr
          class="gutter row"
          aria-orientation="horizontal"
          aria-label="Resize the region detail"
          :aria-valuenow="Math.round(detailShare)"
          :aria-valuemin="SHARE_MIN"
          :aria-valuemax="SHARE_MAX"
          tabindex="0"
          @pointerdown.prevent="dragDetail"
          @keydown="onGutterKey($event, 'detail')"
        >
        <RegionTree
          ref="regions"
          :tree="props.tree"
          :active-index="activeIndex"
          :bands="bands"
          @hover="onTreeHover"
          @pick="select($event, true)"
        />
      </aside>
    </div>
    <HexTip
      v-if="hovered !== null && pointer !== null"
      :tree="props.tree"
      :index="hovered"
      :at="pointer"
      :decode="props.decode"
    />
  </div>
</template>

<style scoped>
.view {
  display: flex;
  flex-direction: column;
  min-height: 0;
  flex: 1;
}
.walk-error code {
  font-family: var(--mono);
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
  /* minmax over 1fr, so the gutter can take width off this column rather than off the window. */
  grid-template-columns: minmax(0, 1fr) auto var(--side, 22rem);
  min-height: 0;
}
/* Sits where the panel border used to, and carries that border itself. */
.gutter {
  background: var(--line);
  border: 0;
  padding: 0;
  margin: 0;
}
.gutter:hover,
.gutter:focus-visible {
  background: var(--accent-rule);
  outline: none;
}
.gutter.col {
  width: 1px;
  cursor: col-resize;
  /* The hit area is wider than the line, without moving the layout. */
  border-left: 3px solid transparent;
  border-right: 3px solid transparent;
  background-clip: padding-box;
  margin: 0 -3px;
  z-index: 1;
}
.gutter.row {
  height: 1px;
  cursor: row-resize;
  border-top: 3px solid transparent;
  border-bottom: 3px solid transparent;
  background-clip: padding-box;
  margin: -3px 0;
  z-index: 1;
}
.left {
  display: flex;
  flex-direction: column;
  min-height: 0;
  min-width: 0;
}
aside {
  background: var(--panel);
  display: flex;
  flex-direction: column;
  min-height: 0;
}
</style>
