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
  <aside class="w-full border-r flex flex-col transition-all duration-300 bg-slate-900 border-slate-700">
    <div class="flex justify-end p-2 border-b border-slate-700" :class="{ 'justify-center': !isOpen }">
      <button class="w-full p-1 rounded transition-colors cursor-pointer hover:bg-slate-700 text-slate-400 hover:text-slate-200" @click="emit('toggle')">
        <Icon
          :name="isOpen ? 'mdi-chevron-left' : 'mdi-chevron-right'"
          class="w-5 h-5"
        />
      </button>
    </div>

    <div v-show="isOpen" class="grid grid-cols-2 gap-2 p-4 border-b border-slate-700">
      <button class="h-6 rounded text-xs transition-colors duration-150 ease-in-out cursor-pointer hover:bg-slate-600 bg-slate-700 text-slate-200" @click="newFile('')">新建</button>
      <button class="h-6 rounded text-xs transition-colors duration-150 ease-in-out cursor-pointer hover:bg-slate-600 bg-slate-700 text-slate-200" @click="openFile">打开</button>
      <button class="h-6 rounded text-xs transition-colors duration-150 ease-in-out cursor-pointer hover:bg-slate-600 bg-slate-700 text-slate-200" @click="saveFile">保存</button>
      <button class="h-6 rounded text-xs transition-colors duration-150 ease-in-out cursor-pointer hover:bg-slate-600 bg-slate-700 text-slate-200" @click="saveFileAs">另存为</button>
      <button class="h-6 rounded text-xs transition-colors duration-150 ease-in-out cursor-pointer hover:bg-slate-600 bg-slate-700 text-slate-200" @click="emit('open-export')">导出MIDI</button>
      <button class="h-6 rounded text-xs transition-colors duration-150 ease-in-out cursor-pointer hover:bg-slate-600 bg-slate-700 text-slate-200" @click="emit('open-help')">帮助</button>
    </div>

    <div class="flex-1 p-2 overflow-auto">
      <button
        v-for="tab in tabs"
        :key="tab.id"
        class="group w-full mb-2 flex justify-between items-center p-2 rounded text-sm text-left overflow-hidden"
        :class="[
          tab.id === activeTab?.id ? 'bg-slate-700 text-slate-100' : 'bg-slate-800 text-slate-300',
          !isOpen ? 'justify-center px-0' : ''
        ]"
        @click="setActive(tab.id)"
      >
        <div class="flex items-center gap-2 min-w-0 flex-1 font-semibold text-sm" :class="!isOpen ? 'justify-center gap-0' : ''">
          <Icon
            name="mdi-file-document-outline"
            class="w-4 h-4 shrink-0"
          />
          <span v-show="isOpen" class="truncate">{{ tab.name }}</span>
          <span v-if="tab.isDirty" class="text-xs shrink-0 text-orange-500">●</span>
        </div>
        <div
          v-show="isOpen"
          class="ml-1 shrink-0 opacity-0 transition-opacity group-hover:opacity-100"
          @click.stop="closeFile(tab.id)"
        >
          <Icon name="mdi-close" class="w-4 h-4" />
        </div>
      </button>
    </div>
  </aside>
</template>
