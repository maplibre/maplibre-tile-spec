<script setup lang="ts">
import { useElementSize } from "@vueuse/core";
import type { FeatureCollection } from "geojson";
import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { meshScene } from "./geometry.ts";

const props = defineProps<{
  tile: FeatureCollection;
  /** Names of the tile's `Meshes` layers. */
  meshLayers: ReadonlySet<string>;
  /** Name of the layer the region tree has scoped to, or null for the whole tile. */
  layer: string | null;
  /** Ground metres per tile unit, which puts heights on the same scale as `x` and `y`. */
  metresPerUnit: number;
}>();

const box = ref<HTMLElement | null>(null);
const { width, height } = useElementSize(box);

const scene = computed(() =>
  meshScene(props.tile, props.meshLayers, props.layer, props.metresPerUnit),
);
const triangles = computed(() => scene.value.hues.length);

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

/** Rebuild the geometry and aim the camera at it from the south-west. */
function rebuild() {
  if (!box.value) return;
  const colours = palette(box.value);
  const { positions, hues } = scene.value;
  const rgb = new Float32Array(positions.length);
  hues.forEach((hue, t) => {
    const c = colours[hue] ?? colours[colours.length - 1];
    for (let corner = 0; corner < 3; corner++)
      c.toArray(rgb, (t * 3 + corner) * 3);
  });
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute("position", new THREE.BufferAttribute(positions, 3));
  geometry.setAttribute("color", new THREE.BufferAttribute(rgb, 3));
  geometry.computeVertexNormals();
  geometry.computeBoundingSphere();
  mesh.geometry.dispose();
  mesh.geometry = geometry;

  const sphere = geometry.boundingSphere;
  if (!sphere || sphere.radius === 0) return render();
  const { center, radius } = sphere;
  camera.near = radius / 100;
  camera.far = radius * 20;
  // Close enough to fill the view with a flat tile, whose bounding sphere is mostly empty.
  const away = new THREE.Vector3(-0.45, 0.55, 0.7).setLength(radius * 2.2);
  camera.position.copy(center).add(away);
  camera.updateProjectionMatrix();
  controls?.target.copy(center);
  controls?.update();
  render();
}

onMounted(() => {
  if (!box.value) return;
  renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
  renderer.setPixelRatio(devicePixelRatio || 1);
  box.value.prepend(renderer.domElement);
  controls = new OrbitControls(camera, renderer.domElement);
  controls.addEventListener("change", render);
  rebuild();
});

onBeforeUnmount(() => {
  controls?.dispose();
  mesh.geometry.dispose();
  renderer?.dispose();
  renderer?.domElement.remove();
});

watch(scene, rebuild);

watch([width, height], ([w, h]) => {
  if (!renderer || w === 0 || h === 0) return;
  renderer.setSize(w, h);
  camera.aspect = w / h;
  camera.updateProjectionMatrix();
  render();
});
</script>

<template>
  <section ref="box" class="mesh">
    <p v-if="triangles === 0" class="none">this tile draws no meshes</p>
    <ul v-if="scene.legend.length > 0" class="legend">
      <li v-for="entry in scene.legend" :key="entry.value">
        <span
          class="swatch"
          :style="{ background: `var(--hue-${entry.hue})` }"
        />
        {{ entry.value }}
      </li>
    </ul>
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
