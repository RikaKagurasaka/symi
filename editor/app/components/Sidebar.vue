<script setup lang="ts">
import { useFileTabs } from "../utils";

const {
  tabs,
  activeTab,
  setActive,
  newFile,
  openFile,
  saveFile,
  saveFileAs,
  closeFile,
} = useFileTabs();

defineProps<{
  isOpen: boolean;
}>();

const emit = defineEmits<{
  (e: "open-export"): void;
  (e: "open-help"): void;
  (e: "toggle"): void;
}>();
</script>

<template>
  <aside class="sidebar" :class="{ 'is-collapsed': !isOpen }">
    <div class="sidebar-toggle-container">
      <button class="toggle-btn" @click="emit('toggle')">
        <Icon
          :name="isOpen ? 'mdi-chevron-left' : 'mdi-chevron-right'"
          class="w-5 h-5"
        />
      </button>
    </div>

    <div v-show="isOpen" class="sidebar-header">
      <button class="action-btn" @click="newFile('')">新建</button>
      <button class="action-btn" @click="openFile">打开</button>
      <button class="action-btn" @click="saveFile">保存</button>
      <button class="action-btn" @click="saveFileAs">另存为</button>
      <button class="action-btn" @click="emit('open-export')">
        导出MIDI
      </button>
      <button class="action-btn" @click="emit('open-help')">帮助</button>
    </div>

    <div class="file-list">
      <button
        v-for="tab in tabs"
        :key="tab.id"
        class="file-item"
        :class="{ active: tab.id === activeTab?.id }"
        @click="setActive(tab.id)"
      >
        <div class="file-name">
          <Icon name="mdi-file-document-outline" class="w-4 h-4 flex-shrink-0" />
          <span v-show="isOpen" class="truncate">{{ tab.name }}</span>
          <span v-if="tab.isDirty" class="dirty-dot">●</span>
        </div>
        <div
          v-show="isOpen"
          class="file-close-btn"
          @click.stop="closeFile(tab.id)"
        >
          <Icon name="mdi-close" class="w-4 h-4" />
        </div>
      </button>
    </div>
  </aside>
</template>

<style lang="css" scoped>
.sidebar {
  @apply w-full border-r flex flex-col transition-all duration-300;
  @apply bg-slate-900 border-slate-700;
}

.sidebar-toggle-container {
  @apply flex justify-end p-2 border-b border-slate-700;
}

.toggle-btn {
  @apply w-full p-1 rounded hover:bg-slate-700 text-slate-400 hover:text-slate-200 transition-colors cursor-pointer;
}

.sidebar-header {
  @apply grid grid-cols-2 gap-2 p-4 border-b;
  @apply border-slate-700;
}

.action-btn {
  @apply h-6 rounded text-xs transition-colors duration-150 ease-in-out cursor-pointer;
  @apply hover:bg-slate-600;
  @apply bg-slate-700 text-slate-200;
}

.file-list {
  @apply flex-1 p-2 overflow-auto;
}

.file-item {
  @apply w-full mb-2 flex justify-between items-center;
  @apply p-2 rounded text-sm text-left overflow-hidden;
  @apply bg-slate-800 text-slate-300;
  &.active {
    @apply bg-slate-700 text-slate-100;
  }
}

.file-name {
  @apply flex items-center gap-2 min-w-0 flex-1;
  @apply font-semibold text-sm;
}

.dirty-dot {
  @apply text-orange-500 text-xs flex-shrink-0;
}

.file-close-btn {
  @apply ml-1 flex-shrink-0 opacity-0 transition-opacity;
}

.file-item:hover .file-close-btn {
  @apply opacity-100;
}

.is-collapsed .sidebar-toggle-container {
  @apply justify-center;
}

.is-collapsed .file-item {
  @apply justify-center p-2 px-0;
}

.is-collapsed .file-name {
  @apply justify-center gap-0;
}
</style>
