<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{
  value: unknown;
  /**
   * Whether `value` came from the dump, which prints a string leaf Rust-debug quoted and
   * leaves every other kind bare. The quotes are what say a value is text at all, so they
   * stay on screen, and only a quoted one is measured.
   */
  quoted?: boolean;
}>();

/** Code points, the same count `blobNote` prints, so a surrogate pair reads as one. */
function count(text: string): number {
  return [...text].length;
}

/** What the quotes hold, falling back to them for an escape JSON does not share, like `\u{1f}`. */
function unquote(raw: string): string {
  try {
    return JSON.parse(raw) as string;
  } catch {
    return raw.slice(1, -1);
  }
}

interface Shown {
  text: string;
  /** A fact about the column rather than text it holds, so it is not printed like text. */
  system: boolean;
  length: number | null;
}

function fromDump(raw: string): Shown {
  if (!(raw.length >= 2 && raw.startsWith('"') && raw.endsWith('"')))
    return { text: raw, system: false, length: null };
  const text = unquote(raw);
  return text === ""
    ? { text: raw, system: true, length: null }
    : { text: raw, system: false, length: count(text) };
}

/**
 * Absent and empty read as tokens rather than as nothing at all, which is what they render
 * as otherwise. A string carries its length after it, telling a padded value from a short
 * one, and a dictionary entry from what points at it.
 */
const shown = computed<Shown>(() => {
  const value = props.value;
  if (value === null || value === undefined)
    return { text: "null", system: true, length: null };
  if (props.quoted) return fromDump(String(value));
  if (value === "") return { text: '""', system: true, length: null };
  return {
    text: String(value),
    system: false,
    length: typeof value === "string" ? count(value) : null,
  };
});
</script>

<template>
  <span :class="{ system: shown.system }">{{ shown.text }}</span>
  <small v-if="shown.length !== null" class="len">({{ shown.length }})</small>
</template>

<style scoped>
/* Not content, and not this app's own prose either: quiet, and slanted away from both. */
.system {
  color: var(--dim);
  font-style: italic;
}
/* A margin rather than a space, which Vue would condense away between the two tags. */
.len {
  margin-left: 0.35em;
  color: var(--dim);
  font-size: 0.9em;
}
</style>
