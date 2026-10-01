<script setup lang="ts">
import { computed, shallowRef, watchEffect } from "vue";
// The same rules the docs site draws its diagrams with, so an inlined SVG is colored here too.
import "../../docs/assets/diagrams.css";
import { type EncodingDoc, encodingDocs, sectionsFor } from "./encodingDocs.ts";

const props = defineProps<{
  /** Section keys, in the order they should read. */
  anchors: readonly string[];
  /** Offer the edit link. A tip takes no pointer, so only a panel says yes. */
  editable?: boolean;
}>();

/** Fetched once and shared: the sections are static, and most sessions open none. */
const docs = shallowRef<Record<string, EncodingDoc>>({});
watchEffect(() => {
  if (props.anchors.length > 0) {
    void encodingDocs().then((loaded) => {
      docs.value = loaded;
    });
  }
});

const sections = computed(() => sectionsFor(props.anchors, docs.value));
</script>

<template>
  <section v-for="doc in sections" :key="doc.key" class="doc">
    <h4>
      <a
        class="title"
        :href="doc.site"
        target="_blank"
        rel="noreferrer"
        :title="`Read ${doc.title} on the spec site`"
        >{{
          doc.title
        }}</a
      >
      <a
        v-if="props.editable"
        class="edit"
        :href="doc.edit"
        target="_blank"
        rel="noreferrer"
        :aria-label="`Edit the ${doc.title} section`"
        >edit</a
      >
    </h4>
    <!-- Built from the encodings page at build time, so it is ours, not a tile's. -->
    <div class="prose" v-html="doc.html"></div>
  </section>
</template>

<style scoped>
.doc + .doc {
  margin-top: var(--pad-tight);
  border-top: 1px solid var(--rule);
  padding-top: var(--pad-tight);
}
h4 {
  display: flex;
  align-items: baseline;
  gap: 0.5rem;
  margin: 0 0 0.2rem;
  font: 600 0.74rem / 1.4 var(--font);
  color: var(--container);
}
.title {
  color: inherit;
  text-decoration: none;
}
.title:hover,
.title:focus-visible {
  text-decoration: underline;
}
.edit {
  font-weight: 400;
  font-size: 0.68rem;
  color: var(--dim);
  /* Out of the way until asked for, since every section would otherwise carry one. */
  visibility: hidden;
}
h4:hover .edit,
.edit:focus-visible {
  visibility: visible;
}
.edit:hover {
  color: var(--accent);
}
.prose {
  /* What the diagram rules read their theme from, on the site, answered from this one. */
  --md-default-fg-color: var(--text);
  --md-default-fg-color--light: var(--muted);
  --md-default-fg-color--lightest: var(--line);
  --md-default-bg-color: var(--panel);
  --md-code-font-family: var(--mono);
  color: var(--text);
}
.prose :deep(p),
.prose :deep(ul),
.prose :deep(ol) {
  margin: 0 0 0.45rem;
}
.prose :deep(pre) {
  margin: 0 0 0.45rem;
  padding: var(--pad-tight);
  background: var(--control);
  border-radius: var(--radius-inline);
  overflow-x: auto;
}
.prose :deep(code) {
  font-family: var(--mono);
  font-size: 0.92em;
}
.prose :deep(blockquote) {
  margin: 0 0 0.45rem;
  padding-left: 0.6rem;
  border-left: 2px solid var(--rule);
  color: var(--muted);
}
.prose :deep(table) {
  border-collapse: collapse;
  font-size: 0.95em;
}
.prose :deep(th),
.prose :deep(td) {
  border: 1px solid var(--rule);
  padding: 0.1rem 0.35rem;
  text-align: left;
}
.prose :deep(a) {
  color: var(--accent);
}
</style>
