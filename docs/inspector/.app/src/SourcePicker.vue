<script setup lang="ts">
import { useFileDialog } from "@vueuse/core";
import { computed, ref } from "vue";
import {
  type FixtureEntry,
  fixtureKey,
  fuzzyMatch,
  groupFixtures,
  starterFixtures,
} from "./fixtures.ts";

const props = defineProps<{
  index: FixtureEntry[];
  /** Index key of the loaded fixture, or null while an upload is shown. */
  current: string | null;
  /** Lay the picker out as the empty state rather than as a bar control. */
  hero?: boolean;
}>();
const emit = defineEmits<{ fixture: [key: string]; upload: [file: File] }>();

const sheet = ref<HTMLDialogElement | null>(null);
const filter = ref("");

/** The bar names the loaded tile, which the hero has none of, so it counts the index instead. */
const browse = computed(() => {
  if (!props.hero) return props.current ?? "choose a fixture...";
  // The index arrives a fetch later, so a count is not available on the first frame.
  if (props.index.length === 0) return "Browse the synthetic fixtures";
  return `Browse one of ${props.index.length} synthetic fixtures`;
});

const starters = computed(() => starterFixtures(props.index));

/** A thousand-odd fixtures make the filter box the sheet's only way in. */
const matches = computed(() => {
  const needle = filter.value.trim().toLowerCase();
  if (needle === "") return props.index;
  return props.index.filter((entry) =>
    fuzzyMatch(fixtureKey(entry).toLowerCase(), needle),
  );
});

const groups = computed(() => groupFixtures(matches.value));

/** Resets on open, so picking the same tile again after re-encoding it still loads it. */
const { open: chooseFile, onChange } = useFileDialog({
  accept: ".mlt",
  multiple: false,
  reset: true,
});

onChange((files) => {
  const file = files?.[0];
  if (file) emit("upload", file);
});

function choose(key: string) {
  emit("fixture", key);
  sheet.value?.close();
}
</script>

<template>
  <div class="source" :class="{ hero: props.hero }">
    <div class="ways">
      <button type="button" class="open" @click="sheet?.showModal()">{{
        browse
      }}</button>
      <button type="button" class="upload" @click="chooseFile()">{{
        props.hero ? "Upload a .mlt file" : "upload .mlt"
      }}</button>
    </div>

    <template v-if="props.hero">
      <div v-if="starters.length" class="starters">
        <h2>or start with</h2>
        <div class="cards">
          <button
            v-for="starter in starters"
            :key="starter.key"
            type="button"
            class="starter"
            @click="choose(starter.key)"
          >
            <span class="title">{{ starter.title }}</span>
            <span class="note">{{ starter.note }}</span>
          </button>
        </div>
      </div>
    </template>

    <dialog ref="sheet" class="sheet" aria-labelledby="fixtures-heading">
      <div class="card">
        <header>
          <div class="titles">
            <h2 id="fixtures-heading">Synthetic fixtures</h2>
            <button
              type="button"
              class="close"
              aria-label="Close"
              @click="sheet?.close()"
              >&times;</button
            >
          </div>
          <input
            v-model="filter"
            class="filter"
            type="search"
            autofocus
            placeholder="Filter by name — fsst, polygon, nested..."
            aria-label="Filter fixtures"
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
          <p v-if="groups.length === 0" class="none"
            >No fixture matches that filter.</p
          >
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
.ways {
  display: flex;
  gap: 0.75rem;
  align-items: center;
  flex-wrap: wrap;
}
.upload,
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
/* The button names the loaded fixture by its index key, which is a path. */
.open {
  font-family: var(--mono);
}

/* The same two buttons as the bar, at the size an empty page can afford. */
.hero {
  box-sizing: border-box;
  flex-direction: column;
  width: min(38rem, 100%);
  gap: 1.3rem;
  font-size: 0.85rem;
}
.hero .upload,
.hero .open {
  padding: 0.8rem 1.3rem;
  font-size: 0.9rem;
}
/* Nothing is loaded, so this button names an action rather than a path. */
.hero .open {
  font-family: var(--font);
  font-weight: 600;
  background: var(--accent);
  color: var(--accent-text);
  border-color: var(--accent-rule);
}
.starters {
  width: 100%;
}
.starters h2 {
  color: var(--muted);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin: 0 0 0.6rem;
  font: 600 0.7rem / 1.4 var(--font);
}
.cards {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(13rem, 1fr));
  gap: 0.6rem;
}
.starter {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
  text-align: left;
  background: var(--control);
  color: var(--text);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  font: inherit;
  padding: var(--pad-tight) 0.9rem;
  cursor: pointer;
}
.starter:hover {
  background: var(--hover);
  border-color: var(--rule);
}
.starter .title {
  font-weight: 600;
}
.starter .note {
  color: var(--muted);
}

.sheet {
  box-sizing: border-box;
  border: none;
  background: none;
  padding: min(4rem, 8vh) var(--pad);
  max-width: none;
  max-height: none;
  width: 100%;
  height: 100%;
}
.sheet::backdrop {
  background: var(--backdrop);
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
  box-shadow: 0 1.5rem 3rem -1rem var(--backdrop);
  font-size: 0.8125rem;
  /* The empty state centres its column, which the sheet is a child of. */
  text-align: left;
}
.card header {
  display: flex;
  flex-direction: column;
  gap: 0.9rem;
  padding: var(--pad);
  border-bottom: 1px solid var(--line);
}
.titles {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--pad-tight);
}
.card h2 {
  margin: 0;
  font: 600 1.0625rem / 1.3 var(--font);
}
.close {
  display: grid;
  place-items: center;
  width: 1.9rem;
  height: 1.9rem;
  flex: none;
  background: none;
  color: var(--muted);
  border: none;
  border-radius: var(--radius);
  font: 1.25rem / 1 var(--font);
  cursor: pointer;
}
.close:hover {
  background: var(--hover);
  color: var(--text);
}
.filter {
  box-sizing: border-box;
  width: 100%;
  background: var(--control);
  color: var(--text);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  font: inherit;
  padding: var(--pad-tight) 0.9rem;
}
.filter:focus-visible {
  outline: 2px solid var(--accent-rule);
  outline-offset: 1px;
  border-color: var(--accent-rule);
}
.groups {
  overflow: auto;
  padding: var(--pad-tight) var(--pad) var(--pad);
}
/* The directory a run of rows belongs to, kept in sight while that run scrolls. */
h3 {
  position: sticky;
  top: 0;
  background: var(--panel);
  color: var(--muted);
  margin: 0;
  padding: 0.9rem 0.75rem 0.4rem;
  font: 600 0.6875rem / 1.4 var(--mono);
}
.entry {
  display: flex;
  align-items: baseline;
  gap: 0.9rem;
  width: 100%;
  background: none;
  border: none;
  border-radius: var(--radius);
  color: var(--text);
  font: inherit;
  font-family: var(--mono);
  text-align: left;
  padding: var(--pad-tight) 0.75rem;
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
  font-size: 0.6875rem;
  font-variant-numeric: tabular-nums;
}
.entry.on .bytes {
  color: var(--accent-text);
}
.none {
  color: var(--muted);
  margin: 0;
  padding: var(--pad) 0.75rem;
}
</style>
