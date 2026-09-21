<script setup lang="ts">
import type { ViewState } from "./hex.ts";

const view = defineModel<ViewState>({ required: true });
const props = defineProps<{ layers: string[] }>();
</script>

<template>
  <div class="knobs">
    <label>
      layer
      <select v-model="view.layer">
        <option :value="null">all ({{ props.layers.length }})</option>
        <option v-for="(layer, i) in props.layers" :key="layer" :value="i">
          {{ i }}
          — {{ layer }}
        </option>
      </select>
      <em class="wasm" title="the only knob the wasm sees">wasm</em>
    </label>
    <label>
      width
      <input v-model.number="view.width" type="range" min="8" max="64" step="4">
      <output>{{ view.width }}</output>
    </label>
    <label>
      data
      <select v-model="view.dataMode">
        <option value="both">both</option>
        <option value="blob">blob</option>
        <option value="decoded">decoded</option>
        <option value="hidden">hidden</option>
      </select>
    </label>
    <label>
      max blob
      <input v-model.number="view.maxBlob" type="number" min="0" step="64">
    </label>
    <label class="check">
      <input v-model="view.showBits" type="checkbox">
      bits
    </label>
  </div>
</template>

<style scoped>
.knobs {
  display: flex;
  gap: 0.75rem;
  align-items: center;
  flex-wrap: wrap;
  font-size: 0.78rem;
  color: var(--muted);
}
label {
  display: flex;
  gap: 0.3rem;
  align-items: center;
}
select,
input[type="number"] {
  background: var(--control);
  color: var(--text);
  border: 1px solid var(--line);
  border-radius: 3px;
  font: inherit;
  padding: 0.05rem 0.2rem;
}
input[type="number"] {
  width: 4.5rem;
}
input[type="range"] {
  width: 5.5rem;
}
output {
  color: var(--text);
  width: 1.5rem;
}
.wasm {
  background: var(--bits-bg);
  color: var(--bits);
  border-radius: 2px;
  padding: 0 0.25rem;
  font-size: 0.68rem;
  font-style: normal;
}
</style>
