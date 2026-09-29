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
          - {{ layer }}
        </option>
      </select>
    </label>
    <button
      type="button"
      class="knob"
      :class="{ on: view.geo }"
      :aria-pressed="view.geo"
      @click="view.geo = !view.geo"
    >
      Geometries
    </button>
    <button
      type="button"
      class="knob"
      :class="{ on: view.colorful }"
      :aria-pressed="view.colorful"
      @click="view.colorful = !view.colorful"
    >
      Tints
    </button>
  </div>
</template>

<style scoped>
.knob {
  background: var(--control);
  color: var(--muted);
  border: 1px solid var(--line);
  border-radius: var(--radius-inline);
  font: inherit;
  font-size: 0.78rem;
  padding: 0.1rem 0.5rem;
  cursor: pointer;
}
.knob:hover {
  background: var(--hover);
}
.knob.on {
  color: var(--text);
  border-color: var(--accent-rule);
}
.knobs {
  display: flex;
  gap: var(--pad);
  align-items: center;
  flex-wrap: wrap;
  font-size: 0.78rem;
  color: var(--muted);
}
label {
  position: relative;
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
  cursor: pointer;
  /* The native caret sits against the border whatever the padding is, so the label draws its own. */
  appearance: none;
  padding: var(--pad-tight) 2rem var(--pad-tight) 0.9rem;
}
/* The select is the label's last child, so the label's right edge is the select's. */
label::after {
  content: "";
  position: absolute;
  right: 0.9rem;
  top: 50%;
  width: 0.4rem;
  height: 0.4rem;
  border-right: 1.5px solid var(--muted);
  border-bottom: 1.5px solid var(--muted);
  transform: translateY(-70%) rotate(45deg);
  pointer-events: none;
}
</style>
