<script setup lang="ts">
import StatusBar from "../components/StatusBar.vue";
import Sidebar from "../components/Sidebar.vue";
import ExportMidiModal from "../components/ExportMidiModal.vue";
import HelpModal from "../components/HelpModal.vue";
import AppToast from "../components/AppToast.vue";
import { useFileTabs } from "../utils";
import { confirm } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import PianoRoll from "~/components/PianoRoll.vue";
import { useVerticalSplit } from "~/composables/useVerticalSplit";
import type { EditorState } from "@codemirror/state";
import type { EditorView } from "@codemirror/view";

const sampleDoc = `(4/4)(200)

lo = {4}G--,-,D-,-,
{4}lo:D,B,A,G,
lo:D[,,],D;D,
lo:D,B,A,G,
= {4}C-,-,G-,-,
E[,,],E,
= {4}C-,-,G-,-,
E,C+,B,A,
= {4}D-,-,G-,-,
F#[,,],F#,
= {4}D-,-,D-,-,
D+,D+,C+,A,
{1}lo:B,

{4}lo:D,B,A,G,
lo:D[,,],D;D,
lo:D,B,A,G,
= {4}C-,-,G-,-,
E[,,],E,
= {4}C-,-,G-,-,
E,C+,B,A,
= {4}D-,-,G-,-,
D+,D+,D+,D+,
= {4}F#--,-,D-,-,
E+,D+,C+,A,
= {4}G--,-,D-,-,
{1}G,

lo = {4}G--,G-,G--,G-
{4}lo:B,B,B,-,
lo:B,B,B,-,
lo:B,D+,G,-;A,
lo:B[,,],,
= {4}C-,C,C-,C
C+,C+,C+,-;C+,
= {4}G--,G-,G--,G-
C+,B,B,B;B,
= {4}A--,A-,A--,A-
B,A,A,B,
= {4}D-,C-,B--,A--
{2}A,D+,

lo = {4}G--,G-,G--,G-
{4}lo:B,B,B,-,
lo:B,B,B,-,
lo:B,D+,G,-;A,
lo:B[,,],,
= {4}C-,C,C-,C
C+,C+,C+,-;C+,
= {4}G--,G-,G--,G-
C+,B,B,B;B,
= {4}D-,D-,E-,F#-,
D+,D+,C+,A,
= {4}G-,,G,,
G,-,G+,,`;

const {
  tabs,
  activeTab,
  setActive,
  newFile,
  updateContent,
  openFile,
  saveFile,
  saveFileAs,
  closeFile,
  hasDirtyTabs,
} = useFileTabs();

const editorState = shallowRef<EditorState | null>(null);
const editorView = shallowRef<EditorView | null>(null);

const exportModalOpen = ref(false);
const helpModalOpen = ref(false);
const toastOpen = ref(false);
const toastType = ref<"success" | "error" | "info">("info");
const toastMessage = ref("");

const isSidebarOpen = ref(true);
const pageRef = ref<HTMLElement | null>(null);
const hasPianoRoll = computed(() => Boolean(activeTab.value));

const { gridTemplateRows, startResize, resizing } = useVerticalSplit({
  containerRef: pageRef,
  enabled: hasPianoRoll,
  storageKey: "index-layout-piano-roll-height",
});

function handleExportResult(payload: { ok: boolean; message: string }) {
  toastType.value = payload.ok ? "success" : "error";
  toastMessage.value = payload.message;
  toastOpen.value = true;
}

const activeContent = computed({
  get: () => activeTab.value?.content ?? "",
  set: (value) => {
    if (!activeTab.value) return;
    updateContent(activeTab.value.id, value);
  },
});

function handleKeydown(event: KeyboardEvent) {
  const isMod = event.metaKey || event.ctrlKey;
  if (!isMod) return;
  const key = event.key.toLowerCase();
  if (key === "s" && event.shiftKey) {
    event.preventDefault();
    void saveFileAs();
    return;
  }
  if (key === "s") {
    event.preventDefault();
    void saveFile();
    return;
  }
  if (key === "o") {
    event.preventDefault();
    void openFile();
    return;
  }
  if (key === "n") {
    event.preventDefault();
    newFile("");
  }
  if (key === "w") {
    event.preventDefault();
    if (activeTab.value) {
      void closeFile(activeTab.value.id);
    }
    return;
  }
}

onMounted(() => {
  window.addEventListener("keydown", handleKeydown);
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", handleKeydown);
});
</script>

<template>
  <div
    ref="pageRef"
    class="index-page w-full h-full grid duration-300 ease-in-out"
    :class="isSidebarOpen ? 'grid-cols-[16rem_1fr]' : 'grid-cols-[3rem_1fr]'"
    :style="{ gridTemplateRows }"
  >
    <Sidebar
      class="row-span-3"
      :is-open="isSidebarOpen"
      @open-export="exportModalOpen = true"
      @open-help="helpModalOpen = true"
      @toggle="isSidebarOpen = !isSidebarOpen"
    />

    <div class="content">
      <StatusBar />
      <Editor
        class="w-full h-full flex-1 overflow-auto bg-[#0f172a]"
        v-if="activeTab"
        v-model="activeContent"
        :file-id="activeTab.id"
        @update:state="editorState = $event"
        @update:view="editorView = $event"
      />
      <div v-else class="empty-state">
        <p>
          请<a @click="newFile('')">新建</a>或<a @click="openFile">打开</a>文件
        </p>
        <p>或者打开<a @click="newFile(sampleDoc)">示例文件</a></p>
      </div>
    </div>
    <div
      v-if="hasPianoRoll"
      class="row-resizer col-start-2 row-start-2"
      :class="{ 'is-resizing': resizing }"
      @pointerdown="startResize"
    />
    <PianoRoll
      v-if="activeTab"
      :editor-state="editorState"
      :editor-view="editorView"
      class="col-start-2 row-start-3 min-h-0 overflow-hidden"
    />
  </div>
  <ExportMidiModal
    v-model="exportModalOpen"
    :file-id="activeTab?.id ?? null"
    :source="activeContent"
    :default-name="activeTab?.name ?? 'Untitled.symi'"
    @export-result="handleExportResult"
  />

  <HelpModal v-model="helpModalOpen" />

  <AppToast v-model="toastOpen" :type="toastType" :message="toastMessage" />
</template>

<style lang="css" scoped>
.index-page {
  @apply bg-slate-800;
}

.content {
  @apply min-w-0 min-h-0 place-self-stretch flex flex-col;
}

.row-resizer {
  @apply h-full w-full col-start-2 row-start-2 cursor-row-resize;
  @apply bg-slate-700/40 hover:bg-slate-600/60 active:bg-slate-500/70;
  touch-action: none;
}

.row-resizer.is-resizing {
  @apply bg-slate-500/70;
}

.empty-state {
  @apply w-full h-full text-center py-8 cursor-default flex flex-col items-center justify-center;
  @apply [&_a]:(underline cursor-pointer text-slate-300 underline-offset-4);
  @apply text-slate-400;
}
</style>
