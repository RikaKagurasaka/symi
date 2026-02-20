<script setup lang="ts">
const props = defineProps<{
  modelValue: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: boolean): void;
}>();

function closeModal() {
  emit("update:modelValue", false);
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
      </div>
    </div>
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
</style>
