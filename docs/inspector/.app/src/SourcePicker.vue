<script setup lang="ts">
import { computed, ref } from "vue";
import { type FixtureEntry, fixtureKey, groupFixtures } from "./fixtures.ts";

const props = defineProps<{
  index: FixtureEntry[];
  /** Index key of the loaded fixture, or null while an upload is shown. */
  current: string | null;
}>();
const emit = defineEmits<{ fixture: [key: string]; upload: [file: File] }>();

const sheet = ref<HTMLDialogElement | null>(null);
const filter = ref("");

/** The index narrowed to the filter, which a thousand-odd fixtures make the sheet's only way in. */
const groups = computed(() => {
  const needle = filter.value.trim().toLowerCase();
  const matches =
    needle === ""
      ? props.index
      : props.index.filter((entry) =>
          fixtureKey(entry).toLowerCase().includes(needle),
        );
  return groupFixtures(matches);
});

function onFile(event: Event) {
  const file = (event.target as HTMLInputElement).files?.[0];
  if (file) emit("upload", file);
}

function choose(key: string) {
  emit("fixture", key);
  sheet.value?.close();
}
</script>

<template>
  <div class="source">
    <label class="upload">
      <input type="file" accept=".mlt" @change="onFile">
      <span>upload .mlt</span>
    </label>
    <span class="hint">or drop a tile anywhere</span>
    <button type="button" class="open" @click="sheet?.showModal()">{{
      props.current ?? "choose a fixture…"
    }}</button>
    <dialog ref="sheet" class="sheet" aria-labelledby="fixtures-heading">
      <div class="card">
        <header>
          <h2 id="fixtures-heading">synthetic fixtures</h2>
          <input
            v-model="filter"
            class="filter"
            type="search"
            placeholder="filter…"
            aria-label="filter fixtures"
          >
          <button type="button" class="close" @click="sheet?.close()"
            >close</button
          >
        </header>
        <div class="groups">
          <section v-for="group in groups" :key="group.label">
            <h3>{{ group.label }}</h3>
            <button
              v-for="entry in group.entries"
              :key="entry.name"
              type="button"
              class="entry"
              :class="{ on: fixtureKey(entry) === props.current }"
              @click="choose(fixtureKey(entry))"
            >
              <span class="name">{{ entry.name }}</span>
              <span class="bytes">{{ entry.bytes }} B</span>
            </button>
          </section>
          <p v-if="groups.length === 0" class="none">no fixture matches</p>
        </div>
      </div>
    </dialog>
  </div>
</template>

<style scoped>
.source {
  display: flex;
  gap: 0.75rem;
  align-items: center;
  flex-wrap: wrap;
  font-size: 0.78rem;
}
.upload input {
  display: none;
}
.upload span,
.open {
  background: var(--control);
  color: var(--text);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  font: inherit;
  padding: var(--pad-tight) 0.9rem;
  cursor: pointer;
  white-space: nowrap;
}
.hint {
  color: var(--muted);
}
.sheet {
  box-sizing: border-box;
  border: none;
  background: none;
  padding: 4rem var(--pad);
  max-width: none;
  max-height: none;
  width: 100%;
  height: 100%;
}
.sheet::backdrop {
  background: #0009;
}
.card {
  box-sizing: border-box;
  margin: auto;
  width: min(38rem, 100%);
  max-height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--panel);
  color: var(--text);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  font-size: 0.78rem;
}
.card header {
  display: flex;
  align-items: center;
  gap: 0.9rem;
  padding: var(--pad);
  border-bottom: 1px solid var(--line);
}
h2 {
  margin: 0;
  font: 600 0.9rem / 1.3 inherit;
}
.filter {
  flex: 1;
  min-width: 0;
  background: var(--control);
  color: var(--text);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  font: inherit;
  padding: var(--pad-tight) 0.9rem;
}
.close {
  background: var(--control);
  color: var(--muted);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  font: inherit;
  padding: var(--pad-tight) 0.9rem;
  cursor: pointer;
}
.groups {
  overflow: auto;
  padding: 0.5rem var(--pad) var(--pad);
}
h3 {
  color: var(--muted);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin: 1.4rem 0 0.5rem;
  padding: 0 0.6rem;
  font:
    600 0.7rem / 1.4 system-ui,
    sans-serif;
}
.entry {
  display: flex;
  gap: 0.6rem;
  width: 100%;
  background: none;
  border: none;
  border-radius: var(--radius);
  color: var(--text);
  font: inherit;
  text-align: left;
  padding: 0.45rem 0.6rem;
  cursor: pointer;
}
.entry:hover {
  background: var(--hover);
}
.entry.on {
  background: var(--accent);
  color: var(--accent-text);
}
.name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.bytes {
  color: var(--dim);
  flex: none;
}
.entry.on .bytes {
  color: var(--accent-text);
}
.none {
  color: var(--muted);
  margin: 1.4rem 0.6rem;
}
</style>
