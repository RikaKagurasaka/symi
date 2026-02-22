<script setup lang="ts">
// Source - https://stackoverflow.com/a/64167032
// Posted by antoni
// Retrieved 2026-02-21, License - CC BY-SA 4.0

import { useClipboard, useColorMode } from "@vueuse/core";
import { useSymiHighlighter } from "../../composables/useSymiHighlighter";

const getSlotChildrenText = (children: any) =>
  children
    .map((node: any) => {
      if (!node.children || typeof node.children === "string")
        return node.children || "";
      else if (Array.isArray(node.children))
        return getSlotChildrenText(node.children);
      else if (node.children.default)
        return getSlotChildrenText(node.children.default());
    })
    .join("");
const { default: defaultSlot } = useSlots();
const slotTexts = computed(
  () => (defaultSlot && getSlotChildrenText(defaultSlot())) || "",
);
defineSlots<{ default?: any }>();
const colorMode = useColorMode();
const { copy, copied } = useClipboard();

const highlighter = await useSymiHighlighter();

const highlightedHtml = computed(() => {
  const source = slotTexts.value;
  const currentTheme = colorMode.value === "dark" ? "symi-dark" : "symi-light";

  if (!source.trim()) {
    console.warn("ProsePre: No source code provided for highlighting.");

    return "";
  }
  try {
    return highlighter.codeToHtml(source, {
      lang: "symi",
      theme: currentTheme,
    });
  } catch (error) {
    console.error("ProsePre: Error highlighting code:", error);
    return "";
  }
});

async function copyCode() {
  if (!slotTexts.value.trim()) {
    return;
  }
  await copy(slotTexts.value);
}
</script>

<template>
  <div class="relative rounded-lg border border-default mb-4">
    <UButton
      class="absolute top-2 right-2 z-10"
      size="xs"
      color="neutral"
      variant="outline"
      :icon="copied ? 'i-lucide-copy-check' : 'i-lucide-copy'"
      @click="copyCode"
    />
    <div v-if="highlightedHtml" class="overflow-x-auto" v-html="highlightedHtml" />
    <pre v-else class="overflow-x-auto"><code>{{ slotTexts }}</code></pre>
  </div>
</template>

<style lang="css" scoped>
:deep(.shiki) {
  margin: 0;
  border-radius: 0.5rem;
  padding: 0.75rem 0.875rem;
  overflow-x: auto;
}
</style>
