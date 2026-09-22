<script setup lang="ts">
import { useElementSize, useScroll } from "@vueuse/core";
import { computed, ref } from "vue";
import type { DumpTree } from "./annotate.ts";
import {
  bandTint,
  fadedFrom,
  hex2,
  hex8,
  type Pointer,
  printable,
  ROW,
  UNANNOTATED,
  type ViewState,
} from "./hex.ts";

const OVERSCAN = 8;

const props = defineProps<{
  tree: DumpTree;
  bytes: Uint8Array;
  owners: Int32Array;
  /** Band per region, or null while the section knob is off. */
  bands: Int32Array | null;
  view: ViewState;
  activeIndex: number | null;
}>();
const emit = defineEmits<{
  /** Where the pointer is too, since the tip that follows it lives beside the map. */
  hover: [index: number | null, at: Pointer | null];
  pick: [index: number];
}>();

const scroller = ref<HTMLElement | null>(null);
const { y: scrollTop } = useScroll(scroller);
/** Border-box, so the height keeps counting the padding the rows scroll through. */
const { height: viewportHeight } = useElementSize(
  scroller,
  { width: 0, height: 600 },
  { box: "border-box" },
);

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
  /** Band this byte sits in, or -1 where no container holds it. */
  band: number;
  starts: boolean;
  faded: boolean;
  opens: boolean;
  closes: boolean;
}

interface Row {
  n: number;
  offset: number;
  cells: Cell[];
  /** Blank cells the short last row needs to keep the ascii column under the one above. */
  pad: number;
  ascii: string;
}

/** Band of the byte at `at`, or -1 past either end of the row and while the knob is off. */
function bandAt(at: number, offset: number, end: number): number {
  const bands = props.bands;
  if (bands === null || at < offset || at >= end) return -1;
  const owner = props.owners[at];
  return owner < 0 ? -1 : bands[owner];
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
      const band = bandAt(at, offset, end);
      cells.push({
        hex: hex2(props.bytes[at]),
        owner,
        band,
        starts: region !== undefined && at === region.offset,
        faded: region !== undefined && at >= fadedFrom(region, props.view),
        opens: band !== bandAt(at - 1, offset, end),
        closes: band !== bandAt(at + 1, offset, end),
      });
      ascii += printable(props.bytes[at]);
    }
    out.push({ n, offset, cells, pad: props.view.width - cells.length, ascii });
  }
  return out;
});

const active = computed(() =>
  props.activeIndex === null
    ? null
    : (props.tree.regions[props.activeIndex] ?? null),
);

/** Byte range the active region lights, which for a container is the whole span it brackets. */
const litSpan = computed<[number, number] | null>(() =>
  active.value
    ? [active.value.offset, active.value.offset + active.value.len]
    : null,
);

/**
 * Only the two ends of a run round off, so a span of bytes reads as one band.
 * A run that outlasts the row rounds at the row's ends too, since it resumes on the next.
 */
function classOf(row: Row, n: number): Record<string, boolean> {
  const cell = row.cells[n];
  const at = row.offset + n;
  const span = litSpan.value;
  const region = props.tree.regions[cell.owner];
  const on = span !== null && at >= span[0] && at < span[1];
  return {
    meta: region?.kind === "meta",
    blob: region?.kind === "dataBlob",
    unannotated: region?.label === UNANNOTATED,
    [`b${bandTint(cell.band)}`]: cell.band >= 0,
    tail: cell.band >= 0 && cell.closes && !on,
    on,
    starts: cell.starts,
    faded: cell.faded,
    opens: on ? n === 0 || at === span[0] : cell.opens,
    closes: on ? n === row.cells.length - 1 || at === span[1] - 1 : cell.closes,
  };
}

/** One listener for the whole map, since a fitted row of a z14 tile is 64 cells of 25,874 rows. */
function ownerOf(event: Event): number | null {
  const owner = (event.target as HTMLElement).dataset?.owner;
  return owner === undefined ? null : Number(owner);
}

/** Every move, not only the ones that change region, since the tip travels with the pointer. */
function onMove(event: MouseEvent) {
  const owner = ownerOf(event);
  const index = owner === null || owner < 0 ? null : owner;
  emit(
    "hover",
    index,
    index === null ? null : { x: event.clientX, y: event.clientY },
  );
}

function onClick(event: MouseEvent) {
  const owner = ownerOf(event);
  if (owner !== null && owner >= 0) emit("pick", owner);
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

defineExpose({ scrollToRegion });
</script>

<template>
  <div ref="scroller" class="map">
    <!-- biome-ignore lint/a11y/useKeyWithClickEvents: the keyboard walks regions with the window's arrow keys, not by clicking a byte -->
    <table
      class="spacer"
      :style="{ height: `${rowCount * ROW}px` }"
      @mousemove="onMove"
      @mouseleave="emit('hover', null, null)"
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
          :style="{ top: `${row.n * ROW}px`, height: `${ROW}px` }"
        >
          <td class="off">{{ hex8(row.offset) }}</td>
          <td
            v-for="(cell, n) in row.cells"
            :key="n"
            class="cell"
            :data-owner="cell.owner"
            :class="classOf(row, n)"
            >{{
              cell.hex
            }}</td
          >
          <td
            v-for="n in row.pad"
            :key="`pad${n}`"
            class="pad"
            aria-hidden="true"
            >{{
              "  "
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
  font-family: var(--mono);
  outline-offset: -2px;
  overflow: auto;
  position: relative;
  padding: 0.9rem 0.9rem 1.5rem;
  font-size: 0.74rem;
}
.spacer {
  position: relative;
}
/* The height is hex.ts's ROW, bound inline, since the virtualizer places every row by it. */
.hexrow {
  position: absolute;
  left: 0;
  right: 0;
  line-height: 16px;
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
  padding: 3px 4px;
  border-left: 1px solid transparent;
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
.cell.b0 {
  background: color-mix(in oklab, var(--hue-0) var(--tint), transparent);
}
.cell.b1 {
  background: color-mix(in oklab, var(--hue-1) var(--tint), transparent);
}
.cell.b2 {
  background: color-mix(in oklab, var(--hue-2) var(--tint), transparent);
}
.cell.b3 {
  background: color-mix(in oklab, var(--hue-3) var(--tint), transparent);
}
.cell.b4 {
  background: color-mix(in oklab, var(--hue-4) var(--tint), transparent);
}
.cell.b5 {
  background: color-mix(in oklab, var(--hue-5) var(--tint), transparent);
}
/* The tint stops short of the next section, so two blocks that touch do not read as one. */
.cell.tail {
  padding-right: 0;
  margin-right: 4px;
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
.cell.opens {
  border-start-start-radius: var(--radius-inline);
  border-end-start-radius: var(--radius-inline);
}
.cell.closes {
  border-start-end-radius: var(--radius-inline);
  border-end-end-radius: var(--radius-inline);
}
.pad {
  padding: 3px 4px;
  border-left: 1px solid transparent;
}
.ascii {
  margin-left: 0.7rem;
  color: var(--dim);
}
</style>
