<script setup lang="ts">
import type { EditorState } from "@codemirror/state";
import type { EditorView } from "@codemirror/view";
import { usePianoRoll } from "~/composables/usePianoRoll";

const { editorState, editorView } = defineProps<{
  editorState: EditorState | null;
  editorView: EditorView | null;
}>();

const scrollerRef = ref<HTMLElement | null>(null);
const svgRef = ref<SVGSVGElement | null>(null);

const { contentWidth, contentHeight } = usePianoRoll({
  editorState: toRef(() => editorState),
  editorView: toRef(() => editorView),
  containerRef: scrollerRef,
  svgRef,
});
    
</script>

<template>
  <div class="piano-roll-container relative w-full h-full min-h-0 overflow-hidden bg-slate-900 text-cyan-300">
    <div ref="scrollerRef" class="piano-roll-scroller w-full h-full overflow-auto" tabindex="0">
      <div
        class="piano-roll-virtual-content"
        :style="{ width: `${contentWidth}px`, height: `${contentHeight}px` }"
      />
    </div>
    <svg ref="svgRef" class="absolute inset-0 pointer-events-none" />
  </div>
</template>

<style lang="css" scoped></style>
