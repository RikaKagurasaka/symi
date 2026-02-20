<script setup lang="ts">
import { EditorState } from "@codemirror/state";
import { EditorView, lineNumbers } from "@codemirror/view";
import { oneDark } from "@codemirror/theme-one-dark";
import { useSmoothWheelScroll } from "~/composables/useSmoothWheelScroll";
import { createBasicEditorExtensions, createViewPlugin } from "../utils/cm";

const props = defineProps<{ modelValue: string; fileId: string }>();
const emit = defineEmits<{
  (e: "update:modelValue", value: string): void;
  (e: "update:state", value: EditorState): void;
  (e: "update:view", value: EditorView | null): void;
}>();

const editorContainer = ref<HTMLElement | null>(null);
const editorScrollRef = ref<HTMLElement | null>(null);
let view: EditorView | null = null;

useSmoothWheelScroll({
  scrollRef: editorScrollRef,
});

const editorBaseExtensions = [lineNumbers(), ...createBasicEditorExtensions()];

const updateListener = EditorView.updateListener.of((update) => {
  emit("update:state", update.state);
  if (!update.docChanged) return;
  emit("update:modelValue", update.state.doc.toString());
});

onMounted(() => {
  if (editorContainer.value) {
    const cmPlugin = createViewPlugin({
      getFileId: () => props.fileId,
    });

    view = new EditorView({
      state: EditorState.create({
        doc: props.modelValue,
        extensions: [
          ...editorBaseExtensions,
          oneDark,
          cmPlugin,
          updateListener,
        ],
      }),
      parent: editorContainer.value,
    });

    editorScrollRef.value = view.scrollDOM;
    emit("update:view", view);
  }
});

watch(
  () => props.modelValue,
  (value) => {
    if (!view) return;
    const current = view.state.doc.toString();
    if (value === current) return;
    const next = value ?? "";
    const prevHead = view.state.selection.main.head;
    const nextAnchor = Math.min(prevHead, next.length);
    view.dispatch({
      changes: { from: 0, to: current.length, insert: next },
      selection: {
        anchor: nextAnchor,
      },
    });
  },
);

onBeforeUnmount(() => {
  emit("update:view", null);
  editorScrollRef.value = null;
  view?.destroy();
  view = null;
});
</script>

<template>
  <div ref="editorContainer" id="editorContainer"></div>
</template>

<style lang="css">
.cm-editor {
  font-size: 20px;
  height: 100%;
}
</style>
