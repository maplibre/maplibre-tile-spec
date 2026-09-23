<script setup lang="ts">
import { useElementSize } from "@vueuse/core";
import { geoIdentity, geoPath } from "d3-geo";
import type { Feature, FeatureCollection } from "geojson";
import { computed, ref, watch } from "vue";
import { extentOf, factsOf, hueOf, layerOf } from "./geometry.ts";

const props = defineProps<{
  /** The decoded tile, or null while none is loaded or it could not be decoded. */
  tile: FeatureCollection | null;
  /** Name of the layer the region tree has scoped to, or null for the whole tile. */
  layer: string | null;
}>();

/**
 * Canvas rather than one SVG node per feature.
 *
 * A real tile carries tens of thousands of features, and at that size any class change
 * makes the browser restyle and repaint every node: half a second to dim a layer. The
 * same scene draws to a canvas in about forty milliseconds, so scoping stays instant.
 */
const box = ref<HTMLElement | null>(null);
const canvas = ref<HTMLCanvasElement | null>(null);
const { width, height } = useElementSize(box);

const extent = computed(() =>
  props.tile === null ? 4096 : extentOf(props.tile),
);

const draw = computed(() =>
  geoPath(geoIdentity()).pointRadius(extent.value / 256),
);

interface Shape {
  path: Path2D;
  /** Tile-space bounds, which keep a hover test off all but a handful of features. */
  x0: number;
  y0: number;
  x1: number;
  y1: number;
  hue: number;
  layer: string;
  /** Open geometry is hit on its stroke; a closed one is hit anywhere inside. */
  closed: boolean;
  feature: Feature;
}

/**
 * One layer's shapes merged into a path per hue, which is what actually gets drawn.
 *
 * Stroking twenty thousand paths one at a time costs about 70ms; merging them and
 * stroking once costs 6, and the merge happens once per tile rather than once per hover.
 */
interface Bucket {
  layer: string;
  /** Everything the layer holds, for the subdued pass, which needs no colour. */
  all: Path2D;
  /** Per hue, so the lit pass keeps a geometry type's own colour. */
  strokes: Map<number, Path2D>;
  /** Closed geometry only, since an open path has nothing to fill. */
  fills: Map<number, Path2D>;
}

const scene = computed(() => {
  const shapes: Shape[] = [];
  const buckets = new Map<string, Bucket>();
  for (const feature of props.tile?.features ?? []) {
    const d = draw.value(feature.geometry);
    if (d === null) continue;
    const path = new Path2D(d);
    const [[x0, y0], [x1, y1]] = draw.value.bounds(feature);
    const hue = hueOf(feature.geometry);
    const layer = layerOf(feature);
    const closed = feature.geometry.type.endsWith("Polygon");
    shapes.push({ path, x0, y0, x1, y1, hue, layer, closed, feature });

    let bucket = buckets.get(layer);
    if (!bucket) {
      bucket = {
        layer,
        all: new Path2D(),
        strokes: new Map(),
        fills: new Map(),
      };
      buckets.set(layer, bucket);
    }
    bucket.all.addPath(path);
    merge(bucket.strokes, hue, path);
    if (closed) merge(bucket.fills, hue, path);
  }
  return { shapes, buckets: [...buckets.values()] };
});

function merge(into: Map<number, Path2D>, hue: number, path: Path2D) {
  const merged = into.get(hue);
  if (merged) merged.addPath(path);
  else {
    const fresh = new Path2D();
    fresh.addPath(path);
    into.set(hue, fresh);
  }
}

const shapes = computed(() => scene.value.shapes);

/** Room around the tile, so its boundary is never flush with the canvas edge. */
const PAD = 8;

/** Tile units to canvas pixels, letterboxed so the extent stays square. */
const fit = computed(() => {
  const span = Math.min(width.value, height.value) - PAD * 2;
  const scale = span > 0 ? span / extent.value : 1;
  return {
    scale,
    dx: (width.value - extent.value * scale) / 2,
    dy: (height.value - extent.value * scale) / 2,
  };
});

const hovered = ref<Shape | null>(null);
const at = ref({ x: 0, y: 0 });

const facts = computed(() =>
  hovered.value === null ? null : factsOf(hovered.value.feature),
);

/** A hovered feature is the whole scope, and it draws on its own over a subdued tile. */
function lit(layer: string): boolean {
  if (hovered.value !== null) return false;
  return props.layer === null || layer === props.layer;
}

/** The palette lives in CSS, so the canvas reads it back rather than repeating it. */
function palette(el: HTMLElement) {
  const style = getComputedStyle(el);
  const of = (name: string) => style.getPropertyValue(name).trim();
  return {
    hues: [0, 1, 2, 3, 4, 5].map((n) => of(`--hue-${n}`)),
    faded: of("--faded") || "#999",
    edge: of("--text") || "#222",
    panel: of("--panel") || "#eee",
    tint: Number.parseFloat(of("--tint")) / 100 || 0.15,
  };
}

let colours = palette(document.documentElement);

function paint(bucket: Bucket, ctx: CanvasRenderingContext2D) {
  ctx.lineWidth = 1.4 / fit.value.scale;
  for (const [hue, path] of bucket.fills) {
    ctx.globalAlpha = colours.tint;
    ctx.fillStyle = colours.hues[hue] ?? colours.faded;
    ctx.fill(path);
  }
  ctx.globalAlpha = 1;
  for (const [hue, path] of bucket.strokes) {
    ctx.strokeStyle = colours.hues[hue] ?? colours.faded;
    ctx.stroke(path);
  }
}

function render() {
  const el = canvas.value;
  const holder = box.value;
  const ctx = el?.getContext("2d");
  if (!el || !holder || !ctx) return;
  colours = palette(holder);

  const dpr = devicePixelRatio || 1;
  el.width = Math.max(1, Math.round(width.value * dpr));
  el.height = Math.max(1, Math.round(height.value * dpr));
  const { scale, dx, dy } = fit.value;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, width.value, height.value);
  ctx.translate(dx, dy);
  ctx.scale(scale, scale);
  ctx.lineJoin = "round";

  // One subdued pass first, so whatever is in scope draws over the top of it.
  const inScope: Bucket[] = [];
  for (const bucket of scene.value.buckets) {
    if (lit(bucket.layer)) inScope.push(bucket);
    else ctx.stroke(bucket.all);
  }
  for (const bucket of inScope) paint(bucket, ctx);

  const one = hovered.value;
  if (one !== null) {
    const colour = colours.hues[one.hue] ?? colours.faded;
    ctx.lineWidth = 2.4 / scale;
    ctx.strokeStyle = colour;
    if (one.closed) {
      ctx.globalAlpha = colours.tint;
      ctx.fillStyle = colour;
      ctx.fill(one.path);
      ctx.globalAlpha = 1;
    }
    ctx.stroke(one.path);
  }

  // The boundary last, and in two tones: geometry runs right up to the extent and past
  // it, so a single colour disappears into whatever it crosses. Dashes over a backing
  // line stay legible against both, and read as a boundary rather than as a feature.
  ctx.lineWidth = 1.5 / scale;
  ctx.setLineDash([]);
  ctx.strokeStyle = colours.panel;
  ctx.strokeRect(0, 0, extent.value, extent.value);
  ctx.setLineDash([6 / scale, 6 / scale]);
  ctx.strokeStyle = colours.edge;
  ctx.strokeRect(0, 0, extent.value, extent.value);
  ctx.setLineDash([]);
}

watch(
  [shapes, width, height, () => props.layer, hovered],
  () => requestAnimationFrame(render),
  { immediate: true },
);

/**
 * The topmost feature the pointer is over, which is the last one drawn.
 *
 * The bounds are compared in tile units, but `isPointInPath` reads its point in the
 * canvas' own device space rather than through the current transform, so the two spaces
 * both have to be handed in.
 */
function pick(
  tile: { x: number; y: number },
  device: { x: number; y: number },
  ctx: CanvasRenderingContext2D,
) {
  // A hair over a handful of pixels, so a hairline is catchable without being grabby.
  ctx.lineWidth = 6 / fit.value.scale;
  const all = shapes.value;
  for (let index = all.length - 1; index >= 0; index--) {
    const shape = all[index];
    if (
      tile.x < shape.x0 ||
      tile.x > shape.x1 ||
      tile.y < shape.y0 ||
      tile.y > shape.y1
    )
      continue;
    if (shape.closed && ctx.isPointInPath(shape.path, device.x, device.y))
      return shape;
    if (ctx.isPointInStroke(shape.path, device.x, device.y)) return shape;
  }
  return null;
}

function onMove(event: PointerEvent) {
  const el = canvas.value;
  const ctx = el?.getContext("2d");
  const holder = box.value;
  if (!el || !ctx || !holder) return;
  const rect = holder.getBoundingClientRect();
  const x = event.clientX - rect.left;
  const y = event.clientY - rect.top;
  at.value = { x, y };
  const { scale, dx, dy } = fit.value;
  const dpr = devicePixelRatio || 1;
  hovered.value = pick(
    { x: (x - dx) / scale, y: (y - dy) / scale },
    { x: x * dpr, y: y * dpr },
    ctx,
  );
}

/** A run over every vertex says nothing a single value would not. */
function spans(count: number): boolean {
  return count > 1;
}
</script>

<template>
  <section ref="box" class="geo" @pointerleave="hovered = null">
    <canvas
      v-show="shapes.length > 0"
      ref="canvas"
      :style="{ width: `${width}px`, height: `${height}px` }"
      @pointermove="onMove"
    />
    <p v-if="shapes.length === 0" class="none">this tile draws no geometry</p>

    <div
      v-if="facts"
      class="tip"
      :style="{ left: `${at.x}px`, top: `${at.y}px` }"
    >
      <p class="head">
        <strong>{{ facts.type }}</strong>
        <span v-if="facts.vertices !== null" class="count">
          {{ facts.vertices }}
          vertices
        </span>
        <span class="layer">{{ facts.layer }}</span>
      </p>
      <dl v-if="facts.properties.length > 0">
        <template v-for="[ name, value ] in facts.properties" :key="name">
          <dt>{{ name }}</dt>
          <dd>{{ value }}</dd>
        </template>
      </dl>
      <table v-for="column in facts.mValues" :key="column.name">
        <caption>
          {{ column.name }}
        </caption>
        <tbody>
          <tr v-for="run in column.runs" :key="run.from">
            <td class="range">
              {{ run.from
              }}<template v-if="spans(run.to - run.from + 1)"
                >..{{ run.to }}</template
              >
            </td>
            <td>{{ run.value }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>

<style scoped>
.geo {
  position: relative;
  min-height: 0;
  display: flex;
  background: var(--panel);
  border-top: 1px solid var(--line);
}
canvas {
  display: block;
  /* Nothing here is clickable, so the pointer never suggests it is. */
  cursor: default;
}
.none {
  margin: auto;
  color: var(--muted);
  font-size: 0.8rem;
}
.tip {
  position: absolute;
  z-index: 2;
  max-width: 22rem;
  max-height: 60%;
  overflow: auto;
  /* Off the pointer, which would otherwise sit on the feature the tip describes. */
  transform: translate(0.9rem, 0.9rem);
  padding: var(--pad-tight);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--control);
  font-size: 0.72rem;
  pointer-events: none;
}
.head {
  margin: 0 0 0.3rem;
  display: flex;
  gap: 0.5rem;
  align-items: baseline;
}
.count,
.layer {
  color: var(--muted);
}
.layer {
  margin-left: auto;
  font-family: var(--mono);
}
dl {
  margin: 0;
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 0 0.6rem;
}
dt {
  color: var(--muted);
}
dd {
  margin: 0;
  font-family: var(--mono);
  overflow-wrap: anywhere;
}
table {
  margin-top: 0.4rem;
  border-collapse: collapse;
  width: 100%;
}
caption {
  text-align: left;
  color: var(--muted);
}
td {
  padding: 0 0.6rem 0 0;
  font-family: var(--mono);
}
.range {
  color: var(--dim);
  white-space: nowrap;
}
</style>
