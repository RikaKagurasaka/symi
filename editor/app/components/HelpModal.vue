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
  <div v-if="modelValue" class="modal-mask" @click.self="closeModal">
    <div class="modal-panel">
      <div class="modal-header">
        <span>快捷键</span>
        <button class="close-btn" @click="closeModal">
          <Icon name="mdi-close" class="w-5 h-5" />
        </button>
      </div>
      <div class="modal-body">
        <h3>通用</h3>
        <ul>
          <li><code>Ctrl+S</code>：保存当前文件</li>
          <li><code>Ctrl+W</code>：关闭当前标签页</li>
        </ul>
        <h3>编辑器</h3>
        <ul>
          <li><code>Ctrl+空格</code>：播放/暂停</li>
          <li><code>Ctrl+Z</code>：撤销</li>
        </ul>
        <h3>钢琴卷帘窗</h3>
        <ul>
          <li><code>鼠标滚轮</code>：水平滚动</li>
          <li><code>Shift+鼠标滚轮</code>：垂直滚动</li>
          <li><code>Ctrl+鼠标滚轮</code>：水平缩放</li>
          <li><code>Alt+鼠标滚轮</code>：垂直缩放</li>
          <li><code>空格</code>：播放/暂停</li>
          <li><code>Ctrl+鼠标左键</code>：跳转光标到点击位置</li>
          <li><code>鼠标中键</code>：拖动视图</li>
        </ul>

        <div class="action-group">
          <button class="action-btn" @click="aboutModalOpen = true">关于</button>
          <button class="action-btn" @click="thirdPartyModalOpen = true">
            第三方库许可说明
          </button>
          <button class="action-btn" @click="void openDocsAndUpdates()">
            查看文档和更新
          </button>
        </div>
      </div>
    </div>

    <About v-model="aboutModalOpen" />
    <ThirdPartyLibs v-model="thirdPartyModalOpen" />
  </div>
</template>

<style lang="css" scoped>
.modal-mask {
  @apply fixed inset-0 z-50 bg-black/40 backdrop-blur-[1px] flex items-center justify-center p-4;
}

.modal-panel {
  @apply w-full max-w-xl rounded-lg border border-slate-700 bg-slate-900 text-slate-200 shadow-xl;
}

.modal-header {
  @apply h-11 px-4 border-b border-slate-700 flex items-center justify-between text-sm font-semibold;
}

.close-btn {
  @apply p-1 rounded text-slate-400 hover:text-slate-200 hover:bg-slate-700 transition-colors;
}

.modal-body {
  @apply p-4 text-sm space-y-3;

  h3 {
    @apply text-slate-100 font-semibold;
  }

  ul {
    @apply list-disc pl-5 space-y-1 text-slate-300;
  }

  code {
    @apply px-1.5 py-0.5 rounded bg-slate-800 text-slate-100;
  }
}

.action-group {
  @apply pt-2 flex flex-wrap gap-2;
}

.action-btn {
  @apply px-3 py-1.5 rounded text-xs bg-slate-700 text-slate-100 hover:bg-slate-600 transition-colors;
}
</style>
