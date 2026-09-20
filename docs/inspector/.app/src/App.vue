<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { type FixtureEntry, loadFixtureIndex } from "./fixtures.ts";

const index = ref<FixtureEntry[]>([]);
const error = ref<string | null>(null);

const directories = computed(() => {
  const groups = new Map<string, { count: number; bytes: number }>();
  for (const entry of index.value) {
    const group = groups.get(entry.directory) ?? { count: 0, bytes: 0 };
    groups.set(entry.directory, {
      count: group.count + 1,
      bytes: group.bytes + entry.bytes,
    });
  }
  return [...groups].map(([directory, group]) => ({ directory, ...group }));
});

onMounted(async () => {
  try {
    index.value = await loadFixtureIndex();
  } catch (cause) {
    error.value = String(cause);
  }
});
</script>

<template>
  <main>
    <h1>MLT Tile Inspector</h1>
    <p v-if="error" role="alert">{{ error }}</p>
    <table v-else>
      <thead>
        <tr>
          <th>directory</th>
          <th>fixtures</th>
          <th>bytes</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="group in directories" :key="group.directory">
          <td>{{ group.directory }}</td>
          <td>{{ group.count }}</td>
          <td>{{ group.bytes }}</td>
        </tr>
      </tbody>
    </table>
  </main>
</template>
