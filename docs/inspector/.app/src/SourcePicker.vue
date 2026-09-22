<script setup lang="ts">
import { useFileDialog } from "@vueuse/core";
import { computed, ref } from "vue";
import {
  FIXTURE_FACETS,
  type FixtureEntry,
  fixtureKey,
  fixtureTags,
  fuzzyMatch,
  groupFixtures,
  matchesTags,
  starterFixtures,
  tagCounts,
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
const picked = ref(new Set<string>());

/** The bar names the loaded tile, which the hero has none of, so it counts the index instead. */
const browse = computed(() => {
  if (!props.hero) return props.current ?? "choose a fixture...";
  // The index arrives a fetch later, so a count is not available on the first frame.
  if (props.index.length === 0) return "Browse the synthetic fixtures";
  return `Browse one of ${props.index.length} synthetic fixtures`;
});

const starters = computed(() => starterFixtures(props.index));

/** The index narrowed by the filter box alone, which is what the chips then count against. */
const searched = computed(() => {
  const needle = filter.value.trim().toLowerCase();
  if (needle === "") return props.index;
  return props.index.filter((entry) =>
    fuzzyMatch(fixtureKey(entry).toLowerCase(), needle),
  );
});

const counts = computed(() => tagCounts(searched.value, picked.value));

/** Both narrowings at once, which a thousand-odd fixtures make the sheet's only way in. */
const matches = computed(() =>
  searched.value.filter((entry) =>
    matchesTags(fixtureTags(entry), picked.value),
  ),
);

const groups = computed(() => groupFixtures(matches.value));

/** A new set each time, since a `Set` mutated in place is not a change Vue sees. */
function toggle(tag: string) {
  const next = new Set(picked.value);
  if (!next.delete(tag)) next.add(tag);
  picked.value = next;
}

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
          <h2 id="fixtures-heading"
            >synthetic fixtures
            <small>{{ matches.length }} of {{ props.index.length }}</small></h2
          >
          <input
            v-model="filter"
            class="filter"
            type="search"
            placeholder="fuzzy filter..."
            aria-label="filter fixtures"
          >
          <button type="button" class="close" @click="sheet?.close()"
            >close</button
          >
        </header>
        <div class="facets">
          <template v-for="facet in FIXTURE_FACETS" :key="facet.label">
            <span class="what">{{ facet.label }}</span>
            <div class="tags">
              <button
                v-for="tag in facet.tags"
                :key="tag"
                type="button"
                class="tag"
                :data-tag="tag"
                :class="{ on: picked.has(tag) }"
                :disabled="!picked.has(tag) && counts.get(tag) === 0"
                :aria-pressed="picked.has(tag)"
                @click="toggle(tag)"
                >{{ tag }} <i>{{ counts.get(tag) }}</i></button
              >
            </div>
          </template>
          <button
            v-if="picked.size"
            type="button"
            class="clear"
            @click="picked = new Set()"
            >clear filters</button
          >
        </div>
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
  padding: 4rem var(--pad);
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
  /* Wide enough that the encoding chips take two rows rather than four. */
  width: min(46rem, 100%);
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
.card h2 {
  margin: 0;
  font: 600 0.9rem / 1.3 inherit;
  text-transform: none;
  letter-spacing: normal;
  color: var(--text);
}
.card h2 small {
  display: block;
  color: var(--dim);
  font-weight: 400;
  font-family: var(--mono);
  font-size: 0.7rem;
}

/* One row per facet: its name in the first column, its chips in the second. */
.facets {
  display: grid;
  grid-template-columns: max-content 1fr;
  align-items: baseline;
  gap: 0.45rem 0.9rem;
  padding: var(--pad);
  border-bottom: 1px solid var(--line);
}
.what {
  color: var(--muted);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  font: 600 0.65rem / 1.6 var(--font);
}
.tags {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
}
.tag {
  background: var(--control);
  color: var(--text);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  font: inherit;
  font-size: 0.72rem;
  padding: 0.25rem 0.6rem;
  cursor: pointer;
  white-space: nowrap;
}
.tag:hover:not(:disabled) {
  border-color: var(--rule);
}
.tag i {
  color: var(--dim);
  font-style: normal;
  font-family: var(--mono);
  font-size: 0.66rem;
}
.tag.on {
  background: var(--accent);
  color: var(--accent-text);
  border-color: var(--accent-rule);
}
.tag.on i {
  color: var(--accent-text);
}
/* Nothing left to narrow, which is worth showing rather than hiding the chip over. */
.tag:disabled {
  color: var(--faded);
  cursor: default;
}
.tag:disabled i {
  color: var(--faded);
}
.clear {
  grid-column: 2;
  justify-self: start;
  background: none;
  border: none;
  border-radius: var(--radius);
  color: var(--muted);
  font: inherit;
  font-size: 0.72rem;
  padding: 0.25rem 0;
  cursor: pointer;
  text-decoration: underline;
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
  font: 600 0.7rem / 1.4 var(--font);
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
  font-family: var(--mono);
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
