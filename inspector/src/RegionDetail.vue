<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { DecodedBlob, DumpTree } from "./annotate.ts";
import { blobChips, blobHidden, blobNote, blobRaw, runs } from "./blob.ts";
import EncodingDocs from "./EncodingDocs.vue";
import { regionAnchors, specPage } from "./encodingDocs.ts";
import { hex2, hexOffset, regionPath } from "./hex.ts";
import { layerSummary } from "./layerSummary.ts";
import ValueText from "./ValueText.vue";

/** Values the detail pane asks for at a time, which is what keeps a 369-blob tile lazy. */
const MAX_VALUES = 64;

/** A text payload crosses whole, and a real tile's string data runs to tens of kilobytes. */
const MAX_CHARS = 256;

/** Each "show more" multiplies both caps by this, for at most `MAX_REVEALS` presses. */
const GROWTH = 8;

/** Past this a stream is still read from its start, and the chips stay few enough to draw. */
const MAX_REVEALS = 2;

const props = defineProps<{
  tree: DumpTree;
  bytes: Uint8Array;
  index: number | null;
  /** False while the pane follows the pointer rather than a selection. */
  sticky: boolean;
  decode: (regionIndex: number, maxValues: number) => DecodedBlob;
}>();

const region = computed(() =>
  props.index === null ? null : (props.tree.regions[props.index] ?? null),
);

const path = computed(() =>
  props.index === null ? [] : regionPath(props.tree.regions, props.index),
);

const layer = computed(() =>
  props.index === null ? null : layerSummary(props.tree.regions, props.index),
);

/** A column's presence bitfield says whether values are there, not what they are. */
const PRESENCE = "present";

/**
 * The blob this region reports: its own, or the one payload it opens.
 *
 * A stream is a header and a payload, so selecting the stream should say what the
 * payload says rather than nothing. Only when there is exactly one payload left once
 * presence is set aside: a column holds several streams, and picking one would be
 * arbitrary.
 */
const payload = computed<number | null>(() => {
  const at = props.index;
  const here = at === null ? undefined : props.tree.regions[at];
  if (here === undefined) return null;
  if (here.blob) return at;
  if (!here.container) return null;
  let only: number | null = null;
  for (
    let i = (at as number) + 1;
    i < props.tree.regions.length && props.tree.regions[i].depth > here.depth;
    i++
  ) {
    const blob = props.tree.regions[i].blob;
    if (!blob || blob.streamType === PRESENCE) continue;
    if (only !== null) return null;
    only = i;
  }
  return only;
});

const blob = computed(() =>
  payload.value === null
    ? null
    : (props.tree.regions[payload.value]?.blob ?? null),
);

/** How many times the decoded section has been grown, which a new payload starts over. */
const reveals = ref(0);
watch(payload, () => {
  reveals.value = 0;
});

const scale = computed(() => GROWTH ** reveals.value);
const maxValues = computed(() => MAX_VALUES * scale.value);
const maxChars = computed(() => MAX_CHARS * scale.value);

const decoded = computed<DecodedBlob | null>(() => {
  if (payload.value === null) return null;
  return props.decode(payload.value, maxValues.value);
});

const chips = computed(() =>
  decoded.value === null ? [] : blobChips(decoded.value, maxChars.value),
);

/** A named value's numbers, kept on a line of their own beneath the names. */
const raw = computed(() =>
  decoded.value === null ? null : blobRaw(decoded.value),
);

/** A text payload is one chip, and a failure one message, so only values fold into runs. */
const groups = computed(() =>
  ["text", "binary", "error"].includes(decoded.value?.kind ?? "")
    ? chips.value.map((chip, from) => ({ chip, count: 1, from }))
    : runs(chips.value),
);

const note = computed(() =>
  decoded.value === null ? "" : blobNote(decoded.value, maxChars.value),
);

/** What "show more" would add: the rest, or the next step of it, whichever is less. */
const more = computed(() => {
  const hidden =
    decoded.value === null ? null : blobHidden(decoded.value, maxChars.value);
  if (hidden === null || reveals.value >= MAX_REVEALS) return null;
  const base = hidden.unit === "chars" ? MAX_CHARS : MAX_VALUES;
  const step = base * scale.value * (GROWTH - 1);
  return { count: Math.min(hidden.count, step), unit: hidden.unit };
});

/** The decoded section already counts them, and for an RLE stream it counts them right. */
const countedBelow = computed(() =>
  ["numbers", "bigints", "bools", "enum"].includes(decoded.value?.kind ?? ""),
);

const span = computed(() => {
  const at = region.value;
  if (!at) return "";
  const last = at.offset + at.len - 1;
  const bufLen = props.tree.bufLen;
  return `${hexOffset(at.offset, bufLen)} ... ${hexOffset(last, bufLen)} (${at.len} B)`;
});

const byte = computed(() =>
  region.value ? props.bytes[region.value.offset] : 0,
);

/** What the encodings page says about this region's fields and its stream. */
const anchors = computed(() =>
  regionAnchors(props.tree.regions, props.index, specPage(props.tree)),
);
</script>

<template>
  <div class="detail">
    <template v-if="region">
      <nav v-if="path.length">{{ path.join(" › ") }}</nav>
      <h2>
        {{ region.label }}
        <em v-if="!props.sticky">click to pin</em>
      </h2>
      <dl>
        <dt>bytes</dt>
        <dd>{{ span }}</dd>
        <template v-if="layer">
          <dt>tag</dt>
          <dd>{{ layer.tag }}</dd>
          <template v-if="layer.extent">
            <dt>extent</dt>
            <dd>{{ layer.extent }}</dd>
          </template>
          <template v-if="layer.features !== null">
            <dt>features</dt>
            <dd>{{ layer.features }}</dd>
          </template>
        </template>
        <template v-if="region.value">
          <dt>value</dt>
          <dd class="value"><ValueText :value="region.value" quoted /></dd>
        </template>
        <template v-if="blob">
          <dt>stream</dt>
          <dd>{{ blob.streamType }}</dd>
          <dt>encoding</dt>
          <dd>{{ blob.logical }} / {{ blob.physical }}</dd>
          <template v-if="!countedBelow">
            <dt>values</dt>
            <dd>{{ blob.numValues }}</dd>
          </template>
          <dt>decodes</dt>
          <dd>{{ blob.hint.kind }}</dd>
        </template>
      </dl>

      <template v-if="layer">
        <h3>columns <small>({{ layer.columns.length }})</small></h3>
        <ol v-if="layer.columns.length" class="columns">
          <li v-for="(column, n) in layer.columns" :key="n">
            {{ column.type }}
            <span v-if="column.name" class="cname">"{{ column.name }}"</span>
          </li>
        </ol>
      </template>

      <template v-if="region.bits.length">
        <h3>bits of 0x{{ hex2(byte) }}</h3>
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
          <i v-for="run in groups" :key="run.from"
            >{{ run.chip
            }}<b
              v-if="run.count > 1"
              :title="`${run.count} identical values in a row`"
              >×{{ run.count }}</b
            ></i
          >
        </div>
        <div v-if="raw" class="values raw" title="as stored">
          <i v-for="run in groups" :key="run.from"
            >{{ raw[run.from]
            }}<b
              v-if="run.count > 1"
              :title="`${run.count} identical values in a row`"
              >×{{ run.count }}</b
            ></i
          >
        </div>
        <div v-if="more || reveals > 0" class="reveal">
          <button v-if="more" type="button" @click="reveals++">
            show {{ more.count }} more {{ more.unit }}
          </button>
          <button v-if="reveals > 0" type="button" @click="reveals = 0">
            show fewer
          </button>
        </div>
      </template>

      <template v-if="anchors.length">
        <h3>encodings</h3>
        <EncodingDocs :anchors="anchors" editable />
      </template>
    </template>
    <p v-else class="idle">
      Hover a byte, or walk the regions with the arrow keys.
    </p>
  </div>
</template>

<style scoped>
/* A fixed share rather than fit-to-content, so the tree below does not move as the pointer travels. */
.detail {
  box-sizing: border-box;
  padding: var(--pad);
  overflow: auto;
  flex: 0 0 30%;
  font-size: 0.76rem;
}
nav {
  color: var(--dim);
  font-family: var(--mono);
  font-size: 0.7rem;
}
h2 {
  font: 600 0.92rem / 1.3 var(--mono);
  color: var(--container);
  margin: 0.05rem 0 0.45rem;
}
h2 em {
  color: var(--dim);
  font: 400 0.68rem / 1 var(--font);
  vertical-align: middle;
  margin-left: 0.35rem;
}
h3 {
  font: 600 0.7rem / 1.4 var(--font);
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
  font-family: var(--mono);
}
dd.value {
  color: var(--value);
}
.columns {
  margin: 0;
  padding: 0;
  list-style: none;
  font-family: var(--mono);
}
.columns li {
  line-height: 1.5;
  overflow-wrap: anywhere;
}
.cname {
  color: var(--value);
}
.bitrow {
  display: flex;
  gap: 0.55rem;
  align-items: baseline;
  line-height: 1.5;
}
.bits {
  font-family: var(--mono);
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
  font-family: var(--mono);
  font-style: normal;
  color: var(--value);
  margin: 0 0.35rem 0.12rem 0;
  /* String data has no spaces to break on, so the chip has to break anywhere. */
  max-width: 100%;
  overflow-wrap: anywhere;
}
.values.raw i {
  color: var(--dim);
}
/* A count, not content: a color of its own keeps it from reading as a value. */
.values b {
  margin-left: 0.2em;
  color: var(--bits);
  font-weight: 600;
}
.values.raw b {
  font-weight: 400;
}
.values.error i,
.values.binary i {
  color: var(--warn);
}
.reveal {
  display: flex;
  gap: 0.75rem;
  margin-top: 0.2rem;
}
.reveal button {
  padding: 0;
  border: 0;
  background: none;
  color: var(--accent-rule);
  font: inherit;
  cursor: pointer;
}
.reveal button:hover {
  text-decoration: underline;
}
.idle {
  color: var(--dim);
  margin: 0.2rem 0;
}
</style>
