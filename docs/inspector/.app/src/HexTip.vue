<script setup lang="ts">
import { useElementSize, useWindowSize } from "@vueuse/core";
import { computed, ref } from "vue";
import type { DecodedBlob, DumpTree } from "./annotate.ts";
import { blobChips, blobNote } from "./blob.ts";
import {
  hex8,
  type Pointer,
  regionDotPath,
  showsDecoded,
  tipPlacement,
  type ViewState,
} from "./hex.ts";

/** Values the tip asks for, which is a glance at a stream rather than the detail pane's page of it. */
const MAX_VALUES = 8;

/** Characters of a text payload the tip shows, which is about one line of it. */
const MAX_CHARS = 48;

const props = defineProps<{
  tree: DumpTree;
  /** Leaf the pointer is over, which the map only ever hands a byte owner. */
  index: number;
  at: Pointer;
  view: ViewState;
  decode: (regionIndex: number, maxValues: number) => DecodedBlob;
}>();

const card = ref<HTMLElement | null>(null);
const { width, height } = useElementSize(card);
const { width: windowWidth, height: windowHeight } = useWindowSize();

const region = computed(() => props.tree.regions[props.index] ?? null);

const path = computed(() => regionDotPath(props.tree.regions, props.index));

const span = computed(() =>
  region.value ? `${region.value.len} B at ${hex8(region.value.offset)}` : "",
);

const decoded = computed<DecodedBlob | null>(() =>
  region.value?.blob && showsDecoded(props.view)
    ? props.decode(props.index, MAX_VALUES)
    : null,
);

const values = computed(() =>
  decoded.value === null ? "" : blobChips(decoded.value, MAX_CHARS).join(", "),
);

const note = computed(() =>
  decoded.value === null ? "" : blobNote(decoded.value, MAX_CHARS),
);

/** Fixed rather than absolute, since the map scrolls under the pointer the tip follows. */
const placed = computed(() => {
  const spot = tipPlacement(
    props.at,
    { width: width.value, height: height.value },
    { width: windowWidth.value, height: windowHeight.value },
  );
  return { left: `${spot.x}px`, top: `${spot.y}px` };
});
</script>

<template>
  <!-- The detail pane says all this to a reader who cannot point at a byte. -->
  <div v-if="region" ref="card" class="tip" :style="placed" aria-hidden="true">
    <code class="path">{{ path }}</code>
    <p class="span">{{ span }}</p>
    <p v-if="region.value" class="value">{{ region.value }}</p>
    <p v-for="field in region.bits" :key="field.hi" class="meaning">{{
      field.meaning
    }}</p>
    <template v-if="decoded">
      <p class="value" :class="decoded.kind">{{ values }}</p>
      <p class="span">{{ note }}</p>
    </template>
  </div>
</template>

<style scoped>
.tip {
  position: fixed;
  z-index: 1;
  pointer-events: none;
  box-sizing: border-box;
  max-width: 22rem;
  padding: var(--pad-tight) 0.8rem;
  background: var(--panel);
  border: 1px solid var(--rule);
  border-radius: var(--radius);
  box-shadow: 0 6px 20px var(--backdrop);
  font-size: 0.74rem;
  line-height: 1.5;
}
.path {
  font-family: var(--mono);
  font-weight: 600;
  color: var(--container);
  /* A deep path has no spaces to break on, so it breaks at the dots instead. */
  overflow-wrap: anywhere;
}
p {
  margin: 0;
}
.span {
  color: var(--dim);
  font-family: var(--mono);
}
.meaning {
  color: var(--text);
}
.value {
  color: var(--value);
  font-family: var(--mono);
  overflow-wrap: anywhere;
}
.value.error,
.value.binary {
  color: var(--warn);
}
</style>
