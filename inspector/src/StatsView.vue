<script setup lang="ts">
import type { FeatureCollection } from "geojson";
import { computed } from "vue";
import type { DumpTree } from "./annotate.ts";
import { formatBytes } from "./bytes.ts";
import { hueOf } from "./geometry.ts";
import {
  CATEGORIES,
  type Category,
  type GeoStat,
  geoStat,
  tileStat,
} from "./tileStats.ts";

const props = defineProps<{
  tree: DumpTree;
  /** The decoded tile, absent when it could not be decoded. */
  tile: FeatureCollection | null;
  /** Index of the layer the pointer is over, among the layers of `tree`. */
  active: number | null;
}>();

const stat = computed(() => tileStat(props.tree));
const geo = computed(() => (props.tile === null ? null : geoStat(props.tile)));

/** Hue slot of the app's block palette each category is drawn in. */
const HUE: Record<Category, number> = {
  geometry: 0,
  properties: 1,
  ids: 5,
  metadata: 3,
};

function share(part: number, whole: number): string {
  if (whole === 0) return "0%";
  const pct = (part / whole) * 100;
  return `${pct < 10 && pct > 0 ? pct.toFixed(1) : Math.round(pct)}%`;
}

function pieces(bytes: Record<Category, number>, whole: number) {
  return CATEGORIES.map((category) => ({
    category,
    len: bytes[category],
    width: whole === 0 ? 0 : (bytes[category] / whole) * 100,
  })).filter((piece) => piece.len > 0);
}

const overall = computed(() => pieces(stat.value.bytes, stat.value.total));

/** The geometry stats of a layer, which only exist once the tile decoded and names it. */
function geoOf(name: string | null): GeoStat | null {
  return name === null ? null : (geo.value?.layers.get(name) ?? null);
}

function perUnit(bytes: number, count: number | null): string | null {
  return count === null || count === 0 ? null : `${(bytes / count).toFixed(1)}`;
}

const types = computed(() => [...(geo.value?.whole.types ?? [])]);

const widest = computed(() =>
  Math.max(1, ...stat.value.layers.map((layer) => layer.len)),
);
</script>

<template>
  <section class="stats" aria-label="Tile statistics">
    <header>
      <strong>{{ formatBytes(stat.total) }}</strong>
      <span class="muted">
        {{ stat.layers.length }}
        {{ stat.layers.length === 1 ? "layer" : "layers" }}
      </span>
      <span v-if="geo" class="muted">
        {{ geo.whole.features }}
        features, {{ geo.whole.vertices }} vertices
      </span>
      <span v-if="stat.unannotated > 0" class="warn">
        {{ formatBytes(stat.unannotated) }}
        unannotated
      </span>
    </header>

    <div class="bar" role="img" aria-label="Bytes by purpose">
      <span
        v-for="piece in overall"
        :key="piece.category"
        :style="{
          width: `${piece.width}%`,
          background: `var(--hue-${HUE[piece.category]})`,
        }"
      />
    </div>
    <ul class="legend">
      <li v-for="category in CATEGORIES" :key="category">
        <i :style="{ background: `var(--hue-${HUE[category]})` }" />
        {{ category }}
        <span class="muted">
          {{ formatBytes(stat.bytes[category]) }}
          {{ share(stat.bytes[category], stat.total) }}
        </span>
      </li>
    </ul>

    <ul v-if="types.length > 0" class="legend types">
      <li v-for="[ type, count ] in types" :key="type">
        <i :style="{ background: `var(--hue-${hueOf({ type } as never)})` }" />
        {{ type }}
        <span class="muted">{{ count }}</span>
      </li>
    </ul>

    <details
      v-for="(layer, at) in stat.layers"
      :key="at"
      class="layer"
      :class="{ active: props.active === at }"
    >
      <summary>
        <span class="name">{{ layer.name ?? `layer ${at}` }}</span>
        <span class="size">{{ formatBytes(layer.len) }}</span>
        <span
          class="bar thin"
          :style="{ width: `${(layer.len / widest) * 100}%` }"
        >
          <span
            v-for="piece in pieces(layer.bytes, layer.len)"
            :key="piece.category"
            :style="{
              width: `${piece.width}%`,
              background: `var(--hue-${HUE[piece.category]})`,
            }"
          />
        </span>
      </summary>
      <dl>
        <template
          v-for="piece in pieces(layer.bytes, layer.len)"
          :key="piece.category"
        >
          <dt>{{ piece.category }}</dt>
          <dd
            >{{ formatBytes(piece.len) }} {{ share(piece.len, layer.len) }}</dd
          >
        </template>
        <template v-if="layer.features !== null">
          <dt>features</dt>
          <dd>
            {{ layer.features }}
            <span v-if="perUnit(layer.len, layer.features)" class="muted">
              {{ perUnit(layer.len, layer.features) }}
              B each
            </span>
          </dd>
        </template>
        <template v-if="geoOf(layer.name)">
          <dt>vertices</dt>
          <dd>
            {{ geoOf(layer.name)?.vertices }}
            <span
              v-if="perUnit(layer.bytes.geometry, geoOf(layer.name)?.vertices ?? null)"
              class="muted"
            >
              {{
                perUnit(
                  layer.bytes.geometry,
                  geoOf(layer.name)?.vertices ?? null,
                )
              }}
              B each
            </span>
          </dd>
        </template>
        <template v-if="layer.extent !== null">
          <dt>extent</dt>
          <dd>{{ layer.extent }}</dd>
        </template>
      </dl>
      <table v-if="layer.columns.length > 0">
        <tbody>
          <tr v-for="(column, i) in layer.columns" :key="i">
            <td>
              <i
                :style="{ background: `var(--hue-${HUE[column.category]})` }"
              />
              {{ column.name || column.type }}
            </td>
            <td class="muted">{{ column.type }}</td>
            <td class="num">{{ formatBytes(column.len) }}</td>
            <td class="num muted">{{ share(column.len, layer.len) }}</td>
          </tr>
        </tbody>
      </table>
    </details>
  </section>
</template>

<style scoped>
.stats {
  min-width: 0;
  min-height: 0;
  overflow: auto;
  padding: var(--pad-tight) var(--pad);
  background: var(--panel);
  border-top: 1px solid var(--line);
  font-size: 0.76rem;
}
header {
  display: flex;
  flex-wrap: wrap;
  gap: 0.2rem 0.8rem;
  align-items: baseline;
  margin-bottom: 0.5rem;
}
.muted {
  color: var(--muted);
}
.warn {
  color: var(--warn);
}
.bar {
  display: flex;
  height: 0.9rem;
  border-radius: 2px;
  overflow: hidden;
  background: var(--line);
}
.bar.thin {
  height: 0.6rem;
  min-width: 2px;
}
.legend {
  display: flex;
  flex-wrap: wrap;
  gap: 0.2rem 0.9rem;
  margin: 0.45rem 0 0.6rem;
  padding: 0;
  list-style: none;
}
.legend.types {
  margin-top: 0;
}
i {
  display: inline-block;
  width: 0.55rem;
  height: 0.55rem;
  margin-right: 0.3rem;
  border-radius: 2px;
}
.layer {
  border-top: 1px solid var(--line);
  padding: 0.3rem 0.5rem;
}
.layer.active {
  border-radius: var(--radius-inline);
  background: var(--accent);
  color: var(--accent-text);
}
summary {
  display: grid;
  grid-template-columns: minmax(4rem, 10rem) 5rem 1fr;
  gap: 0.6rem;
  align-items: center;
  cursor: pointer;
}
.name {
  font-family: var(--mono);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.size {
  text-align: right;
  font-family: var(--mono);
}
dl {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 0 0.8rem;
  margin: 0.4rem 0;
}
dt {
  color: var(--muted);
}
dd {
  margin: 0;
  font-family: var(--mono);
}
table {
  border-collapse: collapse;
  width: 100%;
}
td {
  padding: 0.1rem 0.6rem 0.1rem 0;
  font-family: var(--mono);
}
.num {
  text-align: right;
}
</style>
