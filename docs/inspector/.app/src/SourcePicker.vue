<script setup lang="ts">
import { useFileDialog } from "@vueuse/core";
import { computed, ref } from "vue";
import {
  type AxisRow,
  type AxisValue,
  axisKey,
  axisRows,
  type FixtureEntry,
  fixtureKey,
  fuzzyMatch,
  matchesAxes,
  pinnedRows,
  SECTIONS,
  type Section,
  type SortKey,
  searchVocabulary,
  sectionRows,
  sortFixtures,
  starterFixtures,
  tileAddress,
} from "./fixtures.ts";

const props = defineProps<{
  index: FixtureEntry[];
  /** What names the loaded tile - an index key or an address - or null for an added one. */
  current: string | null;
  /** Lay the picker out as the empty state rather than as a bar control. */
  hero?: boolean;
}>();
const emit = defineEmits<{
  fixture: [key: string];
  file: [file: File];
  url: [address: string];
}>();

const sheet = ref<HTMLDialogElement | null>(null);
const filter = ref("");

/** Picked chips, keyed `<axis>:<value>`, in the URL so a combination can be handed over. */
const filters = defineModel<string[]>("filters", { default: () => [] });
const picked = computed(() => new Set(filters.value));

function toggle(key: string) {
  const next = new Set(filters.value);
  if (!next.delete(key)) next.add(key);
  filters.value = [...next];
}

function clearFacets() {
  filters.value = [];
}

/**
 * The one open section, if any: a second one would push the fixture list off the screen,
 * which is the thing the sheet is for. A closed section still shows its pick count.
 */
const opened = ref<Section | null>(null);

function openSection(section: Section) {
  opened.value = opened.value === section ? null : section;
}

function isOpen(section: Section): boolean {
  return opened.value === section;
}

function picksIn(section: Section): number {
  return sectionRows(rows.value, section).reduce(
    (sum, row) =>
      sum +
      row.values.filter((v) => picked.value.has(axisKey(row, v.value))).length,
    0,
  );
}

/** Coverage reads the same rows, but by what the index holds rather than what is picked. */
const coverage = ref(false);

const columns: SortKey[] = ["name", "bytes"];
const sortKey = ref<SortKey>("name");
const descending = ref(false);

/** A second click on the active column reverses it rather than re-sorting the same way. */
function sortBy(key: SortKey) {
  if (sortKey.value === key) descending.value = !descending.value;
  else {
    sortKey.value = key;
    descending.value = false;
  }
}

function columnName(key: SortKey): string {
  return key === "bytes" ? "size" : "name";
}

/** `aria-sort` belongs to table headers, so a plain button says its direction in its label. */
function sortLabel(key: SortKey): string {
  if (sortKey.value !== key) return `sort by ${columnName(key)}`;
  return `sorted by ${columnName(key)}, ${descending.value ? "descending" : "ascending"}`;
}

/** The bar names the loaded tile, which the hero has none of, so it counts the index instead. */
const browse = computed(() => {
  if (!props.hero) return props.current ?? "choose a fixture...";
  // The index arrives a fetch later, so a count is not available on the first frame.
  if (props.index.length === 0) return "Browse the fixtures";
  return `Browse one of ${props.index.length} fixtures`;
});

const starters = computed(() => starterFixtures(props.index));

/** What the chips alone narrow to, which is what every chip is counted against. */
const chosen = computed(() =>
  props.index.filter((entry) => matchesAxes(entry, picked.value)),
);

/**
 * A thousand-odd fixtures make the filter box the sheet's only way in.
 *
 * Counted over `chosen` rather than over these: the box also searches the vocabulary,
 * and a word like `alp` is in no file name, so counting after it would zero every chip.
 */
const matches = computed(() => {
  const needle = filter.value.trim().toLowerCase();
  if (needle === "") return chosen.value;
  return chosen.value.filter((entry) =>
    fuzzyMatch(fixtureKey(entry).toLowerCase(), needle),
  );
});

const rows = computed(() => axisRows(props.index, chosen.value));

/** Typing offers the chips whose names contain it, which no file-name search would find. */
const hits = computed(() =>
  searchVocabulary(rows.value, filter.value, picked.value),
);

/** The picked chips themselves, so the summary can name and drop them one at a time. */
const chosenChips = computed(() =>
  rows.value.flatMap((row) =>
    row.values
      .filter((value) => picked.value.has(axisKey(row, value.value)))
      .map((value) => ({ row, value })),
  ),
);

function chipClass(row: AxisRow, value: AxisValue) {
  const on = picked.value.has(axisKey(row, value.value));
  return {
    on,
    // Nothing carries it anywhere: a gap in the fixtures, not a miss in this filter.
    gap: !on && value.total === 0,
    // It exists, but not alongside what is already picked, so picking it dead-ends.
    empty: !on && value.total > 0 && value.count === 0,
  };
}

/** A chip that would narrow to nothing is not worth a click, unless it is how you undo one. */
function chipOff(row: AxisRow, value: AxisValue): boolean {
  return !picked.value.has(axisKey(row, value.value)) && value.count === 0;
}

const listed = computed(() =>
  sortFixtures(matches.value, sortKey.value, descending.value),
);

/** Resets on open, so picking the same tile again after re-encoding it still loads it. */
const { open: chooseFile, onChange } = useFileDialog({
  accept: ".mlt",
  multiple: false,
  reset: true,
});

onChange((files) => {
  const file = files?.[0];
  if (file) emit("file", file);
});

function choose(key: string) {
  emit("fixture", key);
  sheet.value?.close();
}

const urlbox = ref<HTMLDialogElement | null>(null);
const typed = ref("");

/**
 * Resolved here rather than after the dialog closes, so a bad address is said so in place.
 * This is the only check: `type="url"` would turn a relative address away before submit
 * ever ran, and a relative one is exactly how a tile beside the page is named.
 */
const address = computed(() =>
  typed.value.trim() === "" ? null : tileAddress(typed.value),
);

function askUrl() {
  typed.value = "";
  urlbox.value?.showModal();
}

function fetchUrl() {
  const found = address.value;
  if (found === null) return;
  emit("url", found);
  urlbox.value?.close();
}
</script>

<template>
  <div class="source" :class="{ hero: props.hero }">
    <div class="ways">
      <button type="button" class="open" @click="sheet?.showModal()">{{
        browse
      }}</button>
      <button type="button" class="add" @click="chooseFile()">Load Tile</button>
      <button type="button" class="add" @click="askUrl()">From URL</button>
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

    <dialog ref="urlbox" class="sheet" aria-labelledby="url-heading">
      <form class="card urlcard" @submit.prevent="fetchUrl">
        <header>
          <div class="titles">
            <h2 id="url-heading">Tile from a URL</h2>
            <button
              type="button"
              class="close"
              aria-label="Close"
              @click="urlbox?.close()"
              >&times;</button
            >
          </div>
          <input
            v-model="typed"
            class="address"
            type="text"
            inputmode="url"
            spellcheck="false"
            autocomplete="off"
            autofocus
            placeholder="https://example.org/14/8298/10748.mlt"
            aria-label="Address of a tile"
          >
          <p class="hint">
            <template v-if="typed.trim() !== '' && address === null"
              >Not a web address this app could fetch a tile from.</template
            >
            <template v-else
              >Fetched by your browser, so the site holding it has to allow
              requests from other sites.</template
            >
          </p>
          <div class="go">
            <button type="submit" class="add" :disabled="address === null"
              >Load</button
            >
          </div>
        </header>
      </form>
    </dialog>

    <dialog ref="sheet" class="sheet" aria-labelledby="fixtures-heading">
      <div class="card">
        <header>
          <div class="titles">
            <h2 id="fixtures-heading">Fixtures</h2>
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
            placeholder="Filter by name - fsst, polygon, nested..."
            aria-label="Filter fixtures"
          >
          <div v-if="hits.length" class="hits">
            <span class="facet-label">matching</span>
            <button
              v-for="hit in hits"
              :key="axisKey(hit.axis, hit.value.value)"
              type="button"
              class="chip hit"
              :class="chipClass(hit.axis, hit.value)"
              :disabled="!coverage && chipOff(hit.axis, hit.value)"
              @click="toggle(axisKey(hit.axis, hit.value.value))"
            >
              <span class="axis">{{ hit.axis.label }}</span>
              {{ hit.value.value
              }}<span class="tally">{{ hit.value.count }}</span>
            </button>
          </div>

          <div v-if="chosenChips.length" class="chosen">
            <span class="facet-label">filtering</span>
            <button
              v-for="chip in chosenChips"
              :key="axisKey(chip.row, chip.value.value)"
              type="button"
              class="chip on"
              :aria-label="`Remove ${chip.row.label} ${chip.value.value}`"
              @click="toggle(axisKey(chip.row, chip.value.value))"
            >
              <span class="axis">{{ chip.row.label }}</span>
              {{ chip.value.value
              }}<span class="drop" aria-hidden="true">&times;</span>
            </button>
          </div>

          <div
            v-for="row in pinnedRows(rows)"
            :key="row.key"
            class="facet pinned"
          >
            <span class="facet-label">{{ row.label }}</span>
            <button
              v-for="option in row.values"
              :key="option.value"
              type="button"
              class="chip"
              :class="chipClass(row, option)"
              :aria-pressed="picked.has(axisKey(row, option.value))"
              :disabled="!coverage && chipOff(row, option)"
              @click="toggle(axisKey(row, option.value))"
            >
              {{ option.value
              }}<span class="tally">{{
                coverage ? option.total : option.count
              }}</span>
            </button>
          </div>

          <div v-for="section in SECTIONS" :key="section" class="section">
            <button
              type="button"
              class="section-head"
              :aria-expanded="isOpen(section)"
              @click="openSection(section)"
            >
              <span class="caret" aria-hidden="true">{{
                isOpen(section) ? "▾" : "▸"
              }}</span>
              {{ section }}
              <span v-if="picksIn(section)" class="badge">{{
                picksIn(section)
              }}</span>
            </button>
            <template v-if="isOpen(section)">
              <div
                v-for="row in sectionRows(rows, section)"
                :key="row.key"
                class="facet"
              >
                <span class="facet-label">{{ row.label }}</span>
                <button
                  v-for="option in row.values"
                  :key="option.value"
                  type="button"
                  class="chip"
                  :class="chipClass(row, option)"
                  :aria-pressed="picked.has(axisKey(row, option.value))"
                  :disabled="!coverage && chipOff(row, option)"
                  :title="
                    option.total === 0
                      ? 'No fixture shows this off yet'
                      : undefined
                  "
                  @click="toggle(axisKey(row, option.value))"
                >
                  {{ option.value
                  }}<span class="tally">{{
                    coverage ? option.total : option.count
                  }}</span>
                </button>
              </div>
            </template>
          </div>

          <p class="tally-line">
            {{ matches.length }}
            of {{ props.index.length }}
            <button
              type="button"
              class="clear"
              :aria-pressed="coverage"
              @click="coverage = !coverage"
              >{{
                coverage ? "counting matches" : "counting coverage"
              }}</button
            >
            <button
              v-if="picked.size"
              type="button"
              class="clear"
              @click="clearFacets"
              >clear filters</button
            >
          </p>
        </header>
        <div class="columns">
          <button
            v-for="column in columns"
            :key="column"
            type="button"
            class="column"
            :class="{ on: sortKey === column, bytes: column === 'bytes' }"
            :aria-label="sortLabel(column)"
            @click="sortBy(column)"
          >
            {{ columnName(column)
            }}<span
              v-if="sortKey === column"
              class="arrow"
              aria-hidden="true"
              >{{
                descending ? "▾" : "▴"
              }}</span
            >
          </button>
        </div>
        <div class="groups">
          <button
            v-for="entry in listed"
            :key="fixtureKey(entry)"
            type="button"
            class="entry"
            :class="{ on: fixtureKey(entry) === props.current }"
            @click="choose(fixtureKey(entry))"
          >
            <span class="name">{{ fixtureKey(entry) }}</span>
            <span class="bytes">{{ entry.bytes }} B</span>
          </button>
          <p v-if="listed.length === 0" class="none"
            >No fixture matches that filter.</p
          >
        </div>
      </div>
    </dialog>
  </div>
</template>

<style scoped>
.columns {
  display: flex;
  gap: 0.4rem;
  padding: var(--pad-tight) var(--pad) 0;
  border-top: 1px solid var(--line);
}
.column {
  background: none;
  border: 0;
  color: var(--muted);
  font: inherit;
  font-size: 0.68rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  cursor: pointer;
  padding: 0.1rem 0.2rem;
}
.column.bytes {
  margin-left: auto;
}
.column.on {
  color: var(--text);
}
.arrow {
  padding-left: 0.2rem;
}
.facet,
.hits,
.chosen {
  display: flex;
  gap: 0.3rem;
  align-items: center;
  flex-wrap: wrap;
  margin-top: 0.5rem;
}
/* The two standing rows sit above the sections, so they are set off from them. */
.hits,
.chosen {
  padding-bottom: 0.45rem;
  border-bottom: 1px solid var(--line);
}
.section {
  margin-top: 0.55rem;
}
.section-head {
  display: flex;
  gap: 0.35rem;
  align-items: center;
  background: none;
  border: 0;
  color: var(--muted);
  font: inherit;
  font-size: 0.68rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  cursor: pointer;
  padding: 0.1rem 0;
}
.section-head:hover {
  color: var(--text);
}
.caret {
  width: 0.7rem;
}
.badge {
  background: var(--accent);
  color: var(--accent-text);
  border-radius: var(--radius-inline);
  font-size: 0.6rem;
  padding: 0 0.3rem;
}
/* Which axis a chip belongs to, for the rows that mix them. */
.chip .axis {
  color: var(--dim);
  font-size: 0.6rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
.chip.on .axis {
  color: inherit;
  opacity: 0.75;
}
.drop {
  color: inherit;
  opacity: 0.75;
  font-size: 0.75rem;
}
/* Nothing in the index has it: a gap in the fixtures rather than a filter miss. */
.chip.gap {
  border-style: dashed;
  color: var(--dim);
}
/* It exists, but not next to what is already picked, so it would narrow to nothing. */
.chip.empty {
  color: var(--dim);
  opacity: 0.55;
}
.chip:disabled {
  cursor: default;
}
.chip:disabled:hover {
  background: var(--control);
}
.facet-label {
  color: var(--muted);
  font-size: 0.66rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  width: 4.6rem;
  flex: 0 0 auto;
}
.chip {
  display: inline-flex;
  gap: 0.3rem;
  align-items: baseline;
  background: var(--control);
  color: var(--text);
  border: 1px solid var(--line);
  border-radius: var(--radius-inline);
  font: inherit;
  font-size: 0.7rem;
  padding: 0.15rem 0.45rem;
  cursor: pointer;
}
.chip:hover {
  background: var(--hover);
}
.chip.on {
  background: var(--accent);
  color: var(--accent-text);
  border-color: var(--accent-rule);
}
.tally {
  color: var(--dim);
  font-size: 0.62rem;
}
.chip.on .tally {
  color: inherit;
}
.tally-line {
  margin: 0.55rem 0 0;
  color: var(--muted);
  font-size: 0.68rem;
}
.clear {
  background: none;
  border: 0;
  color: var(--accent-rule);
  font: inherit;
  font-size: 0.68rem;
  cursor: pointer;
  text-decoration: underline;
  padding: 0 0 0 0.4rem;
}
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
.add,
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
.hero .add,
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
/* Nothing scrolls here, so the card is only as tall as the one question it asks. */
.urlcard {
  max-height: none;
}
.hint {
  margin: 0;
  color: var(--muted);
  font-size: 0.72rem;
}
.go {
  display: flex;
  justify-content: flex-end;
}
.go .add:disabled {
  color: var(--dim);
  cursor: default;
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
.filter,
.address {
  box-sizing: border-box;
  width: 100%;
  background: var(--control);
  color: var(--text);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  font: inherit;
  padding: var(--pad-tight) 0.9rem;
}
.filter:focus-visible,
.address:focus-visible {
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
