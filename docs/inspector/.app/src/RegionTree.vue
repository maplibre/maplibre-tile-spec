<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import type { DumpTree, Region } from "./annotate.ts";
import { ancestors, bandTint, UNANNOTATED } from "./hex.ts";

/** Containers deeper than this start collapsed, which opens a z14 tile on its ten layers. */
const AUTO_OPEN_DEPTH = 1;

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

watch(
  () => props.tree,
  () => {
    collapsed.value = new Set(
      containers(props.tree, (region) => region.depth > AUTO_OPEN_DEPTH),
    );
  },
  { immediate: true },
);

/** The containers that carry a caret, which are the only ones the bar can move. */
const toggleable = computed(() =>
  containers(
    props.tree,
    (region, index) =>
      (props.tree.regions[index + 1]?.depth ?? 0) > region.depth,
  ),
);
/** A half-open tree expands first, so one button covers both directions. */
const opens = computed(() =>
  toggleable.value.some((index) => collapsed.value.has(index)),
);

function toggleAll() {
  collapsed.value = new Set(opens.value ? [] : toggleable.value);
}

interface Node {
  index: number;
  region: Region;
  hasChildren: boolean;
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
  regions.forEach((region, index) => {
    if (region.depth > hideBelow) return;
    hideBelow = Number.POSITIVE_INFINITY;
    const hasChildren =
      region.container && (regions[index + 1]?.depth ?? 0) > region.depth;
    if (region.container && collapsed.value.has(index))
      hideBelow = region.depth;
    const band = bands?.[index] ?? -1;
    const above = out.at(-1);
    if (above) above.bottom = above.band !== band;
    out.push({
      index,
      region,
      hasChildren,
      band,
      top: above === undefined || above.band !== band,
      bottom: true,
    });
  });
  return out;
});

function toggle(index: number) {
  const next = new Set(collapsed.value);
  if (!next.delete(index)) next.add(index);
  collapsed.value = next;
}

/** Picking a header only ever opens its section; the caret is what closes one again. */
function pick(node: Node) {
  if (node.hasChildren && collapsed.value.has(node.index)) {
    const next = new Set(collapsed.value);
    next.delete(node.index);
    collapsed.value = next;
  }
  emit("pick", node.index);
}

/** Opens every container on the way to `index` and scrolls to its row, so revealing from the map cannot land nowhere. */
async function reveal(index: number) {
  if (!props.tree.regions[index]) return;
  const next = new Set(collapsed.value);
  for (const at of ancestors(props.tree.regions, index)) next.delete(at);
  collapsed.value = next;
  await nextTick();
  scrollToRow(index);
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

function containers(
  tree: DumpTree,
  keep: (region: Region, index: number) => boolean,
): number[] {
  return tree.regions.flatMap((region, index) =>
    region.container && keep(region, index) ? [index] : [],
  );
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
          on: node.index === props.activeIndex,
        }"
        :style="{ paddingLeft: `${node.region.depth * 0.8 + 0.3}rem` }"
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
          @click="pick(node)"
        >
          <span class="name">{{ node.region.label }}</span>
          <span class="size">{{ node.region.len }} B</span>
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.tree {
  border-top: 1px solid var(--line);
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
