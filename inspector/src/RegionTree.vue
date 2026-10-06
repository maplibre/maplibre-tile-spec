<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import type { DumpTree, Region } from "./annotate.ts";
import { formatBytes } from "./bytes.ts";
import { ancestors, bandTint, UNANNOTATED } from "./hex.ts";

const props = defineProps<{
  tree: DumpTree;
  activeIndex: number | null;
  /** The same bands the map tints its bytes with, or null while the section knob is off. */
  bands: Int32Array | null;
}>();
const emit = defineEmits<{
  hover: [index: number | null];
  pick: [index: number];
}>();

const collapsed = ref(new Set<number>());
const scroller = ref<HTMLElement | null>(null);

/** One row of the tree, once the wrappers that only pass a region through are gone. */
interface Row {
  /** The outermost region of the chain, which is what the row is keyed and picked by. */
  index: number;
  /** The innermost, whose children the row opens onto. */
  tail: number;
  depth: number;
  hasChildren: boolean;
}

/**
 * The tree with every sole-container child folded into its parent.
 *
 * A column that holds one stream and nothing else costs a row to say so twice, and the
 * reader has to open it to reach anything. The wrapper's own name goes with it: across
 * the fixtures it is only ever `id`, `data` or `vertices`, each of which the parent
 * already implies. `rowOf` maps a region back to the row that swallowed it, since a
 * pick from the map still names the region.
 */
const folded = computed(() => {
  const regions = props.tree.regions;
  const { parent, kids } = shape(regions);
  const rows: Row[] = [];
  /** Which row each region ended up in, so a pick from the map still finds its line. */
  const rowOf = new Int32Array(regions.length);
  regions.forEach((region, index) => {
    const up = parent[index];
    // A wrapper is a container that is its parent's one and only child.
    if (up >= 0 && region.container && kids[up] === 1) {
      rows[rowOf[up]].tail = index;
      rowOf[index] = rowOf[up];
      return;
    }
    rowOf[index] = rows.length;
    rows.push({
      index,
      tail: index,
      depth: up < 0 ? 0 : rows[rowOf[up]].depth + 1,
      hasChildren: false,
    });
  });
  for (const row of rows) row.hasChildren = kids[row.tail] > 0;
  return { rows, rowOf };
});

/** Each region's parent (-1 at the root) and how many children it has, in one pass. */
function shape(regions: Region[]) {
  const parent = new Int32Array(regions.length).fill(-1);
  const kids = new Int32Array(regions.length);
  const open: number[] = [];
  regions.forEach((region, index) => {
    while (open.length && regions[open.at(-1) as number].depth >= region.depth)
      open.pop();
    const up = open.at(-1) ?? -1;
    parent[index] = up;
    if (up >= 0) kids[up] += 1;
    if (region.container) open.push(index);
  });
  return { parent, kids };
}

/**
 * A tile of one layer opens it a single level, which is the whole tile at a glance.
 * Several layers stay shut: opening them all buries the list a reader came to skim.
 */
watch(
  () => props.tree,
  () => {
    const layers = folded.value.rows.filter(
      (row) => row.depth === 0 && row.hasChildren,
    ).length;
    collapsed.value = new Set(
      folded.value.rows
        .filter((row) => row.hasChildren && (layers > 1 || row.depth > 0))
        .map((row) => row.index),
    );
  },
  { immediate: true },
);

/** The rows that carry a caret, which are the only ones the bar can move. */
const toggleable = computed(() =>
  folded.value.rows.filter((row) => row.hasChildren).map((row) => row.index),
);
/** The row the selection lands on, which is the parent of a folded wrapper. */
const active = computed(() =>
  props.activeIndex === null ? null : rowIndex(props.activeIndex),
);
/** A half-open tree expands first, so one button covers both directions. */
const opens = computed(() =>
  toggleable.value.some((index) => collapsed.value.has(index)),
);

function toggleAll() {
  collapsed.value = new Set(opens.value ? [] : toggleable.value);
}

interface Node extends Row {
  region: Region;
  /** Band this row sits in, or -1 for a row no container holds. */
  band: number;
  /** Whether the band's tint rounds off here, so a run of rows reads as one band. */
  top: boolean;
  bottom: boolean;
}

const nodes = computed<Node[]>(() => {
  const out: Node[] = [];
  const regions = props.tree.regions;
  const bands = props.bands;
  let hideBelow = Number.POSITIVE_INFINITY;
  for (const row of folded.value.rows) {
    if (regions[row.index].depth > hideBelow) continue;
    hideBelow = Number.POSITIVE_INFINITY;
    if (row.hasChildren && collapsed.value.has(row.index))
      hideBelow = regions[row.tail].depth;
    const band = bands?.[row.index] ?? -1;
    const above = out.at(-1);
    if (above) above.bottom = above.band !== band;
    out.push({
      ...row,
      region: regions[row.index],
      band,
      top: above === undefined || above.band !== band,
      bottom: true,
    });
  }
  return out;
});

function toggle(index: number) {
  const next = new Set(collapsed.value);
  if (!next.delete(index)) next.add(index);
  collapsed.value = next;
}

/** A double click opens or shuts a section, so a single one can select without moving the list. */
function expand(node: Node) {
  if (node.hasChildren) toggle(node.index);
}

/** Opens every container on the way to `index` and scrolls to its row, so revealing from the map cannot land nowhere. */
async function reveal(index: number) {
  if (!props.tree.regions[index]) return;
  const next = new Set(collapsed.value);
  for (const at of ancestors(props.tree.regions, index)) next.delete(at);
  collapsed.value = next;
  await nextTick();
  scrollToRow(rowIndex(index));
}

/** The row a region shows up in, which is a wrapper's parent once folded. */
function rowIndex(index: number): number {
  const { rows, rowOf } = folded.value;
  return rows[rowOf[index] ?? -1]?.index ?? index;
}

/** Scrolls a row into view, but only when it is not on screen already. */
function scrollToRow(index: number) {
  const el = scroller.value;
  const row = el?.querySelector(`[data-index="${index}"]`);
  if (!el || !row) return;
  const box = el.getBoundingClientRect();
  const at = row.getBoundingClientRect();
  if (at.top >= box.top && at.bottom <= box.bottom) return;
  el.scrollTop += at.top - box.top - el.clientHeight / 3;
}

defineExpose({ reveal });
</script>

<template>
  <div class="tree">
    <div class="treebar">
      <span>regions</span>
      <button
        type="button"
        :disabled="toggleable.length === 0"
        @click="toggleAll"
      >
        {{ opens ? "expand all" : "collapse all" }}
      </button>
    </div>
    <div ref="scroller" class="scroll">
      <div
        v-for="node in nodes"
        :key="node.index"
        :data-index="node.index"
        class="node"
        :class="{
          [`b${bandTint(node.band)}`]: node.band >= 0,
          top: node.top,
          bottom: node.bottom,
          tail: node.band >= 0 && node.bottom,
          container: node.region.container,
          blob: node.region.kind === 'dataBlob',
          unannotated: node.region.label === UNANNOTATED,
          on: node.index === active,
        }"
        :style="{ paddingLeft: `${node.depth * 0.8 + 0.3}rem` }"
      >
        <button
          v-if="node.hasChildren"
          type="button"
          class="caret"
          :aria-expanded="!collapsed.has(node.index)"
          :aria-label="`${collapsed.has(node.index) ? 'expand' : 'collapse'} ${node.region.label}`"
          @click="toggle(node.index)"
        >
          <svg
            class="chevron"
            :class="{ open: !collapsed.has(node.index) }"
            viewBox="0 0 16 16"
            width="10"
            height="10"
            aria-hidden="true"
          >
            <path
              d="M6 3.5 10.5 8 6 12.5"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
        </button>
        <span v-else class="caret" />
        <button
          type="button"
          class="label"
          @mouseenter="emit('hover', node.index)"
          @focus="emit('hover', node.index)"
          @click="emit('pick', node.index)"
          @dblclick="expand(node)"
        >
          <span class="name">{{ node.region.label }}</span>
          <span class="size">{{ formatBytes(node.region.len) }}</span>
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.tree {
  display: flex;
  flex-direction: column;
  min-height: 0;
  flex: 1 1 auto;
}
.treebar {
  display: flex;
  gap: 0.4rem;
  align-items: center;
  padding: var(--pad-tight) var(--pad);
  color: var(--muted);
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}
.treebar span {
  flex: 1;
}
.treebar button {
  background: var(--control);
  color: var(--muted);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  font: inherit;
  padding: 0.15rem 0.6rem;
  text-transform: none;
  letter-spacing: 0;
  cursor: pointer;
}
.treebar button:disabled {
  color: var(--dim);
  cursor: default;
}
.scroll {
  overflow: auto;
  font-family: var(--mono);
  padding: 0 var(--pad) 3rem 0.9rem;
  font-size: 0.74rem;
}
.node {
  display: flex;
  align-items: baseline;
  gap: 0.5rem;
  padding-top: 2px;
  padding-bottom: 2px;
}
.node.top {
  border-start-start-radius: var(--radius-inline);
  border-start-end-radius: var(--radius-inline);
}
.node.bottom {
  border-end-start-radius: var(--radius-inline);
  border-end-end-radius: var(--radius-inline);
}
/* The band stops short of the next section, so two blocks that touch do not read as one. */
.node.tail {
  margin-bottom: 4px;
}
.node.b0 {
  background: color-mix(in oklab, var(--hue-0) var(--tint), transparent);
}
.node.b1 {
  background: color-mix(in oklab, var(--hue-1) var(--tint), transparent);
}
.node.b2 {
  background: color-mix(in oklab, var(--hue-2) var(--tint), transparent);
}
.node.b3 {
  background: color-mix(in oklab, var(--hue-3) var(--tint), transparent);
}
.node.b4 {
  background: color-mix(in oklab, var(--hue-4) var(--tint), transparent);
}
.node.b5 {
  background: color-mix(in oklab, var(--hue-5) var(--tint), transparent);
}
.node:hover,
.node.on {
  border-radius: var(--radius-inline);
}
.node:hover {
  background: var(--hover);
}
.node.on {
  background: var(--accent);
}
/* An icon rather than a glyph, so the arrow centres on the row instead of sitting on its baseline. */
.caret {
  width: 0.9rem;
  flex: none;
  align-self: center;
  display: flex;
  justify-content: center;
  background: none;
  border: none;
  color: var(--muted);
  cursor: pointer;
  padding: 0;
}
.chevron {
  display: block;
  transition: transform 120ms ease;
}
.chevron.open {
  transform: rotate(90deg);
}
.label {
  display: flex;
  gap: 0.3rem;
  align-items: baseline;
  flex: 1;
  min-width: 0;
  background: none;
  border: none;
  font: inherit;
  color: var(--text);
  cursor: pointer;
  padding: 0.16rem 0.3rem;
  text-align: left;
  /* A double click toggles the section, so it must not also select the label text. */
  user-select: none;
}
.name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.node.container .label {
  color: var(--container);
}
.node.blob .label {
  color: var(--blob);
}
.node.unannotated .label {
  color: var(--warn);
}
.node.on .label {
  color: var(--accent-text);
}
.size {
  margin-left: auto;
  flex: none;
  color: var(--dim);
  font-size: 0.68rem;
}
</style>
