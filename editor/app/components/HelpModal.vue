<script setup lang="ts">
import { openUrl } from "@tauri-apps/plugin-opener";
import About from "./About.vue";
import ThirdPartyLibs from "./ThirdPartyLibs.vue";

const props = defineProps<{
  modelValue: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: boolean): void;
}>();

function closeModal() {
  emit("update:modelValue", false);
}

const thirdPartyModalOpen = ref(false);
const aboutModalOpen = ref(false);

async function openDocsAndUpdates() {
  await openUrl("https://symi.rika.link");
}

useEventListener(window, "keydown", (event: KeyboardEvent) => {
  if (!props.modelValue) return;
  if (event.key !== "Escape") return;
  event.preventDefault();
  closeModal();
});
</script>

<template>
  <div v-if="modelValue" class="fixed inset-0 z-50 bg-black/40 backdrop-blur-[1px] flex items-center justify-center p-4" @click.self="closeModal">
    <div class="w-full max-w-xl rounded-lg border border-slate-700 bg-slate-900 text-slate-200 shadow-xl">
      <div class="h-11 px-4 border-b border-slate-700 flex items-center justify-between text-sm font-semibold">
        <span>快捷键</span>
        <button class="p-1 rounded text-slate-400 hover:text-slate-200 hover:bg-slate-700 transition-colors" @click="closeModal">
          <Icon name="mdi-close" class="w-5 h-5" />
        </button>
      </div>
      <div class="p-4 text-sm space-y-3">
        <h3 class="text-slate-100 font-semibold">通用</h3>
        <ul class="list-disc pl-5 space-y-1 text-slate-300">
          <li><code class="px-1.5 py-0.5 rounded bg-slate-800 text-slate-100">Ctrl+S</code>：保存当前文件</li>
          <li><code class="px-1.5 py-0.5 rounded bg-slate-800 text-slate-100">Ctrl+W</code>：关闭当前标签页</li>
        </ul>
        <h3 class="text-slate-100 font-semibold">编辑器</h3>
        <ul class="list-disc pl-5 space-y-1 text-slate-300">
          <li><code class="px-1.5 py-0.5 rounded bg-slate-800 text-slate-100">Ctrl+空格</code>：播放/暂停</li>
          <li><code class="px-1.5 py-0.5 rounded bg-slate-800 text-slate-100">Ctrl+Z</code>：撤销</li>
        </ul>
        <h3 class="text-slate-100 font-semibold">钢琴卷帘窗</h3>
        <ul class="list-disc pl-5 space-y-1 text-slate-300">
          <li><code class="px-1.5 py-0.5 rounded bg-slate-800 text-slate-100">鼠标滚轮</code>：水平滚动</li>
          <li><code class="px-1.5 py-0.5 rounded bg-slate-800 text-slate-100">Shift+鼠标滚轮</code>：垂直滚动</li>
          <li><code class="px-1.5 py-0.5 rounded bg-slate-800 text-slate-100">Ctrl+鼠标滚轮</code>：水平缩放</li>
          <li><code class="px-1.5 py-0.5 rounded bg-slate-800 text-slate-100">Alt+鼠标滚轮</code>：垂直缩放</li>
          <li><code class="px-1.5 py-0.5 rounded bg-slate-800 text-slate-100">空格</code>：播放/暂停</li>
          <li><code class="px-1.5 py-0.5 rounded bg-slate-800 text-slate-100">Ctrl+鼠标左键</code>：跳转光标到点击位置</li>
          <li><code class="px-1.5 py-0.5 rounded bg-slate-800 text-slate-100">鼠标中键</code>：拖动视图</li>
        </ul>

        <div class="pt-2 flex flex-wrap gap-2">
          <button class="px-3 py-1.5 rounded text-xs bg-slate-700 text-slate-100 hover:bg-slate-600 transition-colors" @click="aboutModalOpen = true">关于</button>
          <button class="px-3 py-1.5 rounded text-xs bg-slate-700 text-slate-100 hover:bg-slate-600 transition-colors" @click="thirdPartyModalOpen = true">
            第三方库许可说明
          </button>
          <button class="px-3 py-1.5 rounded text-xs bg-slate-700 text-slate-100 hover:bg-slate-600 transition-colors" @click="void openDocsAndUpdates()">
            查看文档和更新
          </button>
        </div>
      </div>
    </div>

    <About v-model="aboutModalOpen" />
    <ThirdPartyLibs v-model="thirdPartyModalOpen" />
  </div>
</template>
