<script setup lang="ts">
import { useElementSize, useMutationObserver } from "@vueuse/core";
import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import type { MltColumnTile } from "./annotate.ts";
import { type MeshColumn, meshColumns, meshScene } from "./geometry.ts";

const props = defineProps<{
  /** The decoded columns, whose `Meshes` layers the view draws. */
  columns: MltColumnTile;
  /** Name of the layer the region tree has scoped to, or null for the whole tile. */
  layer: string | null;
  /** Ground metres per tile unit, which puts heights on the same scale as `x` and `y`. */
  metresPerUnit: number;
}>();

const box = ref<HTMLElement | null>(null);
const { width, height } = useElementSize(box);

/** The m-value columns the meshes can be coloured by. */
const columns = computed(() => meshColumns(props.columns, props.layer));
const picked = ref<MeshColumn | null>(null);
/** The picked column while the meshes have it, else the first, a triangle column when there is one. */
const column = computed(
  () =>
    columns.value.find(
      (c) => c.name === picked.value?.name && c.scope === picked.value?.scope,
    ) ??
    columns.value[0] ??
    null,
);
const pickedIndex = computed({
  get: () => (column.value ? columns.value.indexOf(column.value) : -1),
  set: (at: number) => {
    picked.value = columns.value[at] ?? null;
  },
});

const labelOf = (c: MeshColumn) => `${c.name} (per ${c.scope})`;

const scene = computed(() =>
  meshScene(props.columns, props.layer, props.metresPerUnit, column.value),
);
const triangles = computed(() => scene.value.positions.length / 9);

/** The palette lives in CSS, so the view reads it back rather than repeating it. */
function palette(el: HTMLElement): THREE.Color[] {
  const style = getComputedStyle(el);
  const of = (name: string, fallback: string) =>
    new THREE.Color(style.getPropertyValue(name).trim() || fallback);
  return [
    ...[0, 1, 2, 3, 4, 5].map((n) => of(`--hue-${n}`, "#888")),
    of("--faded", "#999"),
  ];
}

let renderer: THREE.WebGLRenderer | null = null;
let controls: OrbitControls | null = null;
const world = new THREE.Scene();
const camera = new THREE.PerspectiveCamera(40, 1, 1, 10);
const mesh = new THREE.Mesh(
  new THREE.BufferGeometry(),
  // Front faces only, so a triangle wound the wrong way shows as a hole.
  new THREE.MeshLambertMaterial({ vertexColors: true, side: THREE.FrontSide }),
);
world.add(mesh);
world.add(new THREE.HemisphereLight(0xffffff, 0x556677, 1.6));
const sun = new THREE.DirectionalLight(0xffffff, 1.3);
sun.position.set(-1, 2, 1.5);
world.add(sun);

function render() {
  renderer?.render(world, camera);
}

/** Paint each corner the CSS hue of its value in the picked column, so the mesh matches the legend. */
function paint() {
  if (!box.value) return;
  const colours = palette(box.value);
  const { hues } = scene.value;
  const rgb = new Float32Array(hues.length * 3);
  hues.forEach((hue, corner) => {
    (colours[hue] ?? colours[colours.length - 1]).toArray(rgb, corner * 3);
  });
  mesh.geometry.setAttribute("color", new THREE.BufferAttribute(rgb, 3));
}

/** Rebuild the geometry and aim the camera at it from the south-west. */
function rebuild() {
  if (!box.value) return;
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute(
    "position",
    new THREE.BufferAttribute(scene.value.positions, 3),
  );
  geometry.computeVertexNormals();
  geometry.computeBoundingBox();
  mesh.geometry.dispose();
  mesh.geometry = geometry;
  paint();
  frame(new THREE.Vector3(-0.45, 0.55, 0.7));
}

/**
 * Aim the camera at the meshes from `toward`, just far enough that every corner of their
 * bounding box fits both the vertical and the horizontal field of view.
 */
function frame(toward: THREE.Vector3) {
  const bounds = mesh.geometry.boundingBox;
  if (!bounds || bounds.isEmpty()) return render();
  const center = bounds.getCenter(new THREE.Vector3());
  const back = toward.clone().normalize();
  const forward = back.clone().negate();
  const right = new THREE.Vector3()
    .crossVectors(forward, camera.up)
    .normalize();
  const up = new THREE.Vector3().crossVectors(right, forward);
  const tanV = Math.tan(THREE.MathUtils.degToRad(camera.fov) / 2);
  const tanH = tanV * camera.aspect;
  let distance = 0;
  for (const x of [bounds.min.x, bounds.max.x])
    for (const y of [bounds.min.y, bounds.max.y])
      for (const z of [bounds.min.z, bounds.max.z]) {
        const corner = new THREE.Vector3(x, y, z).sub(center);
        const depth = corner.dot(forward);
        distance = Math.max(
          distance,
          Math.abs(corner.dot(right)) / tanH - depth,
          Math.abs(corner.dot(up)) / tanV - depth,
        );
      }
  distance *= 1.05;
  camera.position.copy(center).addScaledVector(back, distance);
  controls?.target.copy(center);
  controls?.update();
  clip();
  render();
}

/** Put the clipping planes around the meshes as seen from where the camera is now. */
function clip() {
  const bounds = mesh.geometry.boundingBox;
  if (!bounds || bounds.isEmpty()) return;
  const eye = camera.position;
  camera.updateMatrixWorld();
  const forward = camera.getWorldDirection(new THREE.Vector3());
  // The camera clips by depth along its view, which is least at a corner of the box.
  let nearest = Infinity;
  let farthest = 0;
  for (const x of [bounds.min.x, bounds.max.x])
    for (const y of [bounds.min.y, bounds.max.y])
      for (const z of [bounds.min.z, bounds.max.z]) {
        const corner = new THREE.Vector3(x, y, z);
        nearest = Math.min(nearest, corner.clone().sub(eye).dot(forward));
        farthest = Math.max(farthest, eye.distanceTo(corner));
      }
  // With a corner beside or behind the eye, the nearest surface can be arbitrarily close, so the near plane only keeps depth precision.
  camera.near = Math.max(nearest * 0.9, farthest / 10_000);
  camera.far = farthest * 1.1;
  camera.updateProjectionMatrix();
}

onMounted(() => {
  if (!box.value) return;
  renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
  renderer.setPixelRatio(devicePixelRatio || 1);
  box.value.prepend(renderer.domElement);
  controls = new OrbitControls(camera, renderer.domElement);
  controls.addEventListener("change", () => {
    clip();
    render();
  });
  rebuild();
});

onBeforeUnmount(() => {
  controls?.dispose();
  mesh.geometry.dispose();
  renderer?.dispose();
  renderer?.domElement.remove();
});

watch(() => [props.columns, props.layer, props.metresPerUnit], rebuild);
// Another column only repaints, so the camera stays where it was turned to.
watch(column, () => {
  paint();
  render();
});

// The docs toggle and the OS scheme both end in `data-theme`, which swaps the CSS hues.
useMutationObserver(
  document.documentElement,
  () => {
    paint();
    render();
  },
  { attributeFilter: ["data-theme"] },
);

watch([width, height], ([w, h]) => {
  if (!renderer || w === 0 || h === 0) return;
  renderer.setSize(w, h);
  camera.aspect = w / h;
  // Refit from where the camera looks now, so a narrower panel never crops the meshes.
  frame(camera.position.clone().sub(controls?.target ?? new THREE.Vector3()));
});
</script>

<template>
  <section ref="box" class="mesh">
    <p v-if="triangles === 0" class="none">this tile draws no meshes</p>
    <div v-if="column" class="legend">
      <label v-if="columns.length > 1" class="pick">
        <select v-model="pickedIndex" aria-label="colour the meshes by">
          <option
            v-for="(each, at) in columns"
            :key="labelOf(each)"
            :value="at"
          >
            {{ labelOf(each) }}
          </option>
        </select>
      </label>
      <p v-else class="title">{{ labelOf(column) }}</p>
      <ul>
        <li v-for="entry in scene.legend" :key="entry.value">
          <span
            class="swatch"
            :style="{ background: `var(--hue-${entry.hue})` }"
          />
          {{ entry.value }}
        </li>
      </ul>
    </div>
  </section>
</template>

<style scoped>
.mesh {
  position: relative;
  min-height: 0;
  min-width: 0;
  overflow: hidden;
  background: var(--panel);
  border-top: 1px solid var(--line);
}
.mesh :deep(canvas) {
  display: block;
  cursor: grab;
}
.none {
  position: absolute;
  inset: 0;
  margin: auto;
  height: fit-content;
  text-align: center;
  color: var(--muted);
  font-size: 0.8rem;
}
.legend {
  position: absolute;
  top: var(--pad-tight);
  left: var(--pad-tight);
  margin: 0;
  padding: var(--pad-tight);
  list-style: none;
  border-radius: var(--radius);
  background: var(--control);
  font-size: 0.72rem;
  pointer-events: none;
}
.title {
  margin: 0 0 var(--pad-tight);
  color: var(--muted);
}
.pick {
  position: relative;
  display: block;
  margin-bottom: var(--pad-tight);
  pointer-events: auto;
}
select {
  background: var(--panel);
  color: var(--text);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  font: inherit;
  cursor: pointer;
  /* The native caret sits against the border whatever the padding is, so the label draws its own. */
  appearance: none;
  padding: 0.2rem 1.6rem 0.2rem 0.5rem;
}
.pick::after {
  content: "";
  position: absolute;
  right: 0.6rem;
  top: 50%;
  width: 0.35rem;
  height: 0.35rem;
  border-right: 1.5px solid var(--muted);
  border-bottom: 1.5px solid var(--muted);
  transform: translateY(-70%) rotate(45deg);
  pointer-events: none;
}
.legend ul {
  margin: 0;
  padding: 0;
  list-style: none;
}
.legend li {
  display: flex;
  align-items: center;
  gap: 0.4rem;
}
.swatch {
  width: 0.7rem;
  height: 0.7rem;
  border-radius: 0.2rem;
}
</style>
