<script setup lang="ts">
import { computed } from "vue";
import { type FixtureEntry, groupFixtures } from "./fixtures.ts";

const props = defineProps<{
  index: FixtureEntry[];
  /** Index key of the loaded fixture, or null while an upload is shown. */
  current: string | null;
}>();
const emit = defineEmits<{ fixture: [key: string]; upload: [file: File] }>();

const groups = computed(() => groupFixtures(props.index));

function onFile(event: Event) {
  const file = (event.target as HTMLInputElement).files?.[0];
  if (file) emit("upload", file);
}
</script>

<template>
  <div class="source">
    <label class="upload">
      <input type="file" accept=".mlt" @change="onFile">
      <span>upload .mlt</span>
    </label>
    <span class="hint">or drop a tile anywhere</span>
    <select
      :value="props.current ?? ''"
      aria-label="synthetic fixture"
      @change="emit('fixture', ($event.target as HTMLSelectElement).value)"
    >
      <option value="" disabled>choose a fixture…</option>
      <optgroup v-for="group in groups" :key="group.label" :label="group.label">
        <option
          v-for="entry in group.entries"
          :key="entry.name"
          :value="`${entry.directory}/${entry.name}`"
        >
          {{ entry.name }}
          · {{ entry.bytes }} B
        </option>
      </optgroup>
    </select>
  </div>
</template>

<style scoped>
.source {
  display: flex;
  gap: 0.6rem;
  align-items: center;
  flex-wrap: wrap;
  font-size: 0.78rem;
}
.upload input {
  display: none;
}
.upload span {
  border: 1px solid var(--line);
  border-radius: 3px;
  padding: 0.15rem 0.5rem;
  cursor: pointer;
  white-space: nowrap;
  color: var(--text);
}
.hint {
  color: var(--muted);
}
select {
  background: var(--control);
  color: var(--text);
  border: 1px solid var(--line);
  border-radius: 3px;
  font: inherit;
  padding: 0.1rem 0.2rem;
  max-width: 19rem;
}
</style>
