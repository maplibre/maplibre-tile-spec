<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import type { DumpTree } from "./annotate.ts";
import {
  fadedFrom,
  hex2,
  hex8,
  printable,
  UNANNOTATED,
  type ViewState,
} from "./hex.ts";

/** Row height in pixels, which the stylesheet below pins and the virtualizer counts in. */
const ROW = 19;
const OVERSCAN = 8;

const props = defineProps<{
  tree: DumpTree;
  bytes: Uint8Array;
  owners: Int32Array;
  view: ViewState;
  activeIndex: number | null;
}>();
const emit = defineEmits<{
  hover: [index: number | null];
  pick: [index: number];
}>();

const scroller = ref<HTMLElement | null>(null);
let hovered: number | null = null;
const scrollTop = ref(0);
const viewportHeight = ref(600);

const rowCount = computed(() =>
  Math.ceil(props.tree.bufLen / props.view.width),
);

const slice = computed(() => {
  const first = Math.max(0, Math.floor(scrollTop.value / ROW) - OVERSCAN);
  const rows = Math.ceil(viewportHeight.value / ROW) + OVERSCAN * 2;
  return { first, last: Math.min(rowCount.value, first + rows) };
});

interface Cell {
  hex: string;
  owner: number;
  starts: boolean;
  faded: boolean;
}

interface Row {
  n: number;
  offset: number;
  cells: Cell[];
  ascii: string;
}

const rows = computed<Row[]>(() => {
  const out: Row[] = [];
  for (let n = slice.value.first; n < slice.value.last; n++) {
    const offset = n * props.view.width;
    const end = Math.min(offset + props.view.width, props.tree.bufLen);
    const cells: Cell[] = [];
    let ascii = "";
    for (let at = offset; at < end; at++) {
      const owner = props.owners[at];
      const region = props.tree.regions[owner];
      cells.push({
        hex: hex2(props.bytes[at]),
        owner,
        starts: region !== undefined && at === region.offset,
        faded: region !== undefined && at >= fadedFrom(region, props.view),
      });
      ascii += printable(props.bytes[at]);
    }
    out.push({ n, offset, cells, ascii });
  }
  return out;
});

const active = computed(() =>
  props.activeIndex === null
    ? null
    : (props.tree.regions[props.activeIndex] ?? null),
);

/** A container owns no bytes of its own, so it lights the whole span it brackets. */
function lit(owner: number, at: number): boolean {
  const region = active.value;
  if (!region) return false;
  if (region.container)
    return at >= region.offset && at < region.offset + region.len;
  return owner === props.activeIndex;
}

function classOf(cell: Cell, at: number): Record<string, boolean> {
  const region = props.tree.regions[cell.owner];
  return {
    meta: region?.kind === "meta",
    blob: region?.kind === "dataBlob",
    unannotated: region?.label === UNANNOTATED,
    on: lit(cell.owner, at),
    starts: cell.starts,
    faded: cell.faded,
  };
}

/** One listener for the whole map, since a fitted row of a z14 tile is 64 cells of 25,874 rows. */
function ownerOf(event: Event): number | null {
  const owner = (event.target as HTMLElement).dataset?.owner;
  return owner === undefined ? null : Number(owner);
}

function onMove(event: MouseEvent) {
  const owner = ownerOf(event);
  if (owner !== hovered) {
    hovered = owner;
    emit("hover", owner === null || owner < 0 ? null : owner);
  }
}

function onClick(event: MouseEvent) {
  const owner = ownerOf(event);
  if (owner !== null && owner >= 0) emit("pick", owner);
}

function onScroll(event: Event) {
  const el = event.target as HTMLElement;
  scrollTop.value = el.scrollTop;
  viewportHeight.value = el.clientHeight;
}

/** Scrolls a region into view, but only when it is not on screen already. */
function scrollToRegion(index: number) {
  const el = scroller.value;
  const region = props.tree.regions[index];
  if (!el || !region) return;
  const top = Math.floor(region.offset / props.view.width) * ROW;
  const seen = el.scrollTop;
  if (top >= seen && top <= seen + el.clientHeight - ROW * 2) return;
  el.scrollTop = Math.max(0, top - el.clientHeight / 3);
}

let observer: ResizeObserver | null = null;

onMounted(() => {
  if (!scroller.value) return;
  viewportHeight.value = scroller.value.clientHeight;
  observer = new ResizeObserver(() => {
    if (scroller.value) viewportHeight.value = scroller.value.clientHeight;
  });
  observer.observe(scroller.value);
});

onBeforeUnmount(() => observer?.disconnect());

defineExpose({ scrollToRegion });
</script>

<template>
  <div ref="scroller" class="map" @scroll="onScroll">
    <!-- biome-ignore lint/a11y/useKeyWithClickEvents: the keyboard walks regions with the window's arrow keys, not by clicking a byte -->
    <table
      class="spacer"
      :style="{ height: `${rowCount * ROW}px` }"
      @mousemove="onMove"
      @mouseleave="emit('hover', null)"
      @click="onClick"
    >
      <caption class="sr-only"
        >tile bytes</caption
      >
      <tbody>
        <tr
          v-for="row in rows"
          :key="row.n"
          class="hexrow"
          :style="{ top: `${row.n * ROW}px` }"
        >
          <td class="off">{{ hex8(row.offset) }}</td>
          <td
            v-for="(cell, n) in row.cells"
            :key="n"
            class="cell"
            :data-owner="cell.owner"
            :class="classOf(cell, row.offset + n)"
            >{{
              cell.hex
            }}</td
          >
          <td class="ascii">{{ row.ascii }}</td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<style scoped>
.map {
  flex: 1;
  outline-offset: -2px;
  overflow: auto;
  position: relative;
  padding: 0.9rem var(--pad) 1.5rem;
  font-size: 0.74rem;
}
.spacer {
  position: relative;
}
.hexrow {
  position: absolute;
  left: 0;
  right: 0;
  height: 19px;
  display: flex;
  align-items: center;
  white-space: pre;
}
.off {
  color: var(--dim);
  margin-right: 0.55rem;
}
.cell {
  color: var(--blob);
  cursor: pointer;
  padding: 1px 2px 1px 1px;
  border-left: 1px solid transparent;
  border-radius: var(--radius-inline);
}
.cell.meta {
  color: var(--text);
}
.cell.starts {
  border-left-color: var(--rule);
}
.cell.faded {
  color: var(--faded);
}
.cell.unannotated {
  color: var(--warn);
  background: var(--warn-bg);
}
.cell.on {
  background: var(--accent);
  color: var(--accent-text);
  border-left-color: var(--accent);
}
.cell.starts.on {
  border-left-color: var(--accent-rule);
}
.ascii {
  margin-left: 0.7rem;
  color: var(--dim);
}
</style>
