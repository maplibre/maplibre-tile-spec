<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { DumpTree, Region } from "./annotate.ts";
import { ancestors, UNANNOTATED } from "./hex.ts";

/** Containers deeper than this start collapsed, which opens a z14 tile on its ten layers. */
const AUTO_OPEN_DEPTH = 1;

const props = defineProps<{ tree: DumpTree; activeIndex: number | null }>();
const emit = defineEmits<{
  hover: [index: number | null];
  pick: [index: number];
}>();

const collapsed = ref(new Set<number>());

watch(
  () => props.tree,
  () => {
    collapsed.value = new Set(
      containers(props.tree, (region) => region.depth > AUTO_OPEN_DEPTH),
    );
  },
  { immediate: true },
);

interface Node {
  index: number;
  region: Region;
  hasChildren: boolean;
}

const nodes = computed<Node[]>(() => {
  const out: Node[] = [];
  const regions = props.tree.regions;
  let hideBelow = Number.POSITIVE_INFINITY;
  regions.forEach((region, index) => {
    if (region.depth > hideBelow) return;
    hideBelow = Number.POSITIVE_INFINITY;
    const hasChildren =
      region.container && (regions[index + 1]?.depth ?? 0) > region.depth;
    if (region.container && collapsed.value.has(index))
      hideBelow = region.depth;
    out.push({ index, region, hasChildren });
  });
  return out;
});

function toggle(index: number) {
  const next = new Set(collapsed.value);
  if (!next.delete(index)) next.add(index);
  collapsed.value = next;
}

/** Opens every container on the way to `index`, so revealing from the map cannot land nowhere. */
function reveal(index: number) {
  if (!props.tree.regions[index]) return;
  const next = new Set(collapsed.value);
  for (const at of ancestors(props.tree.regions, index)) next.delete(at);
  collapsed.value = next;
}

function containers(
  tree: DumpTree,
  keep: (region: Region) => boolean,
): number[] {
  return tree.regions.flatMap((region, index) =>
    region.container && keep(region) ? [index] : [],
  );
}

defineExpose({ reveal });
</script>

<template>
  <div class="tree">
    <div class="treebar">
      <span>regions</span>
      <button type="button" @click="collapsed = new Set()">expand</button>
      <button
        type="button"
        @click="collapsed = new Set(containers(props.tree, () => true))"
      >
        collapse
      </button>
    </div>
    <div class="scroll">
      <div
        v-for="node in nodes"
        :key="node.index"
        class="node"
        :class="{
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
          >{{
            collapsed.has(node.index) ? "▸" : "▾"
          }}</button
        >
        <span v-else class="caret" />
        <button
          type="button"
          class="label"
          @mouseenter="emit('hover', node.index)"
          @focus="emit('hover', node.index)"
          @click="emit('pick', node.index)"
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
.scroll {
  overflow: auto;
  padding: 0 var(--pad) 3rem 0.9rem;
  font-size: 0.74rem;
}
.node {
  display: flex;
  align-items: baseline;
  gap: 0.3rem;
  border-radius: var(--radius-inline);
}
.node:hover {
  background: var(--hover);
}
.node.on {
  background: var(--accent);
}
.caret {
  width: 0.9rem;
  flex: none;
  background: none;
  border: none;
  color: var(--muted);
  cursor: pointer;
  font: inherit;
  padding: 0;
  text-align: left;
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
