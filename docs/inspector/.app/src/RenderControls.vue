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
  </div>
</template>

<style scoped>
.knobs {
  display: flex;
  gap: var(--pad);
  align-items: center;
  flex-wrap: wrap;
  font-size: 0.78rem;
  color: var(--muted);
}
label {
  display: flex;
  gap: 0.4rem;
  align-items: center;
}
select {
  background: var(--control);
  color: var(--text);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  font: inherit;
  padding: var(--pad-tight) 0.7rem;
}
</style>
