<script setup lang="ts">
import { computed } from "vue";
import type { DecodedBlob, DumpTree } from "./annotate.ts";
import { hex2, hex8, regionPath, showsDecoded, type ViewState } from "./hex.ts";

/** Values the detail pane asks for at a time, which is what keeps a 369-blob tile lazy. */
const MAX_VALUES = 64;

const props = defineProps<{
  tree: DumpTree;
  bytes: Uint8Array;
  index: number | null;
  /** False while the pane follows the pointer rather than a selection. */
  sticky: boolean;
  view: ViewState;
  decode: (regionIndex: number, maxValues: number) => DecodedBlob;
}>();

const region = computed(() =>
  props.index === null ? null : (props.tree.regions[props.index] ?? null),
);

const path = computed(() =>
  props.index === null ? [] : regionPath(props.tree.regions, props.index),
);

const childCount = computed(() => {
  const at = props.index;
  if (at === null || !props.tree.regions[at]?.container) return 0;
  const depth = props.tree.regions[at].depth;
  let count = 0;
  for (
    let i = at + 1;
    i < props.tree.regions.length && props.tree.regions[i].depth > depth;
    i++
  ) {
    count += 1;
  }
  return count;
});

const decoded = computed<DecodedBlob | null>(() => {
  if (props.index === null || !region.value?.blob || !showsDecoded(props.view))
    return null;
  return props.decode(props.index, MAX_VALUES);
});

const chips = computed(() => {
  const blob = decoded.value;
  if (!blob) return [];
  switch (blob.kind) {
    case "numbers":
    case "bigints":
      return blob.values.map(String);
    case "bools":
      return blob.values.map((value) => (value ? "1" : "0"));
    case "text":
      return [blob.value];
    case "binary":
      return [`${blob.len} binary bytes`];
    case "error":
      return [blob.message];
  }
});

const note = computed(() => {
  const blob = decoded.value;
  if (!blob) return "";
  if (blob.kind === "error") return "undecodable";
  if (blob.kind === "text" || blob.kind === "binary") return blob.kind;
  return blob.truncatedFrom === null
    ? `${blob.values.length} values`
    : `${blob.values.length} of ${blob.truncatedFrom} values`;
});

const byte = computed(() =>
  region.value ? props.bytes[region.value.offset] : 0,
);
</script>

<template>
  <div class="detail">
    <template v-if="region">
      <nav v-if="path.length">{{ path.join(" › ") }}</nav>
      <h2>
        {{ region.label }}
        <em v-if="!props.sticky">hovering</em>
      </h2>
      <dl>
        <dt>bytes</dt>
        <dd>
          {{ hex8(region.offset) }}
          … {{ hex8(region.offset + region.len - 1) }} · {{ region.len }} B
        </dd>
        <template v-if="region.container">
          <dt>holds</dt>
          <dd>{{ childCount }} regions</dd>
        </template>
        <template v-if="region.value">
          <dt>value</dt>
          <dd class="value">{{ region.value }}</dd>
        </template>
        <template v-if="region.blob">
          <dt>stream</dt>
          <dd>{{ region.blob.streamType }}</dd>
          <dt>encoding</dt>
          <dd>{{ region.blob.logical }} / {{ region.blob.physical }}</dd>
          <dt>values</dt>
          <dd>{{ region.blob.numValues }} · {{ region.blob.hint.kind }}</dd>
        </template>
      </dl>

      <template v-if="region.bits.length && props.view.showBits">
        <h3>bits of {{ hex2(byte) }}</h3>
        <div v-for="field in region.bits" :key="field.hi" class="bitrow">
          <span class="bits"
            ><i
              v-for="n in 8"
              :key="n"
              :class="{ in: 8 - n <= field.hi && 8 - n >= field.lo }"
              >{{
                (byte >> (8 - n)) & 1
              }}</i
            ></span
          >
          <span class="meaning">{{ field.meaning }}</span>
        </div>
      </template>

      <template v-if="decoded">
        <h3>decoded <small>{{ note }}</small></h3>
        <div class="values" :class="decoded.kind">
          <i v-for="(chip, n) in chips" :key="n">{{ chip }}</i>
        </div>
      </template>
    </template>
    <p v-else class="idle">Hover a byte, or walk the regions with ↑ / ↓.</p>
  </div>
</template>

<style scoped>
/* flex-shrink stays 0: the region tree below must not squeeze this card down to its first line. */
.detail {
  padding: 0.55rem 0.7rem 0.7rem;
  overflow: auto;
  flex: 0 0 auto;
  max-height: 58%;
  font-size: 0.76rem;
}
nav {
  color: var(--dim);
  font-size: 0.7rem;
}
h2 {
  font:
    600 0.92rem / 1.3 ui-monospace,
    monospace;
  color: var(--container);
  margin: 0.05rem 0 0.45rem;
}
h2 em {
  color: var(--dim);
  font:
    400 0.68rem / 1 system-ui,
    sans-serif;
  vertical-align: middle;
  margin-left: 0.35rem;
}
h3 {
  font:
    600 0.7rem / 1.4 system-ui,
    sans-serif;
  color: var(--muted);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin: 0.85rem 0 0.3rem;
}
h3 small {
  text-transform: none;
  letter-spacing: 0;
  color: var(--dim);
  font-weight: 400;
}
dl {
  display: grid;
  grid-template-columns: 4.4rem 1fr;
  gap: 0.12rem 0.45rem;
  margin: 0;
}
dt {
  color: var(--muted);
}
dd {
  margin: 0;
  color: var(--text);
}
dd.value {
  color: var(--value);
}
.bitrow {
  display: flex;
  gap: 0.55rem;
  align-items: baseline;
  line-height: 1.5;
}
.bits i {
  font-style: normal;
  color: var(--faded);
}
.bits i.in {
  color: var(--bits);
  font-weight: 600;
}
.meaning {
  color: var(--text);
}
.values i {
  display: inline-block;
  font-style: normal;
  color: var(--value);
  margin: 0 0.35rem 0.12rem 0;
}
.values.error i,
.values.binary i {
  color: var(--warn);
}
.idle {
  color: var(--dim);
  margin: 0.2rem 0;
}
</style>
