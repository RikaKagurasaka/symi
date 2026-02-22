<script setup lang="ts">
import { openUrl } from "@tauri-apps/plugin-opener";
import pkg from "../../package.json";

const props = defineProps<{
  modelValue: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: boolean): void;
}>();

const currentVersion = computed(() => pkg.version ?? "unknown");
const repoUrl = "https://github.com/RikaKagurasaka/symi";
const discordUrl = "https://discord.gg/pyZYtqXjeB";
const logoUrl = "/logo.svg";

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
        <span>关于 Symi</span>
        <button class="close-btn" @click="closeModal">
          <Icon name="mdi-close" class="w-5 h-5" />
        </button>
      </div>

      <div class="modal-body">
        <div class="hero">
          <img :src="logoUrl" alt="Symi Logo" class="logo" />
          <div class="hero-text">
            <h3>Symi Editor</h3>
            <p>用于微分音乐创作的 Symi 可视化编辑器。</p>
            <div class="meta-row">
              <span class="meta-chip">v{{ currentVersion }}</span>
              <span class="meta-chip">Apache 2.0</span>
            </div>
          </div>
        </div>

        <p>
          Symi（Synthesized Microtone）是一种可用于微分音乐的标记语言，受
          simai 语启发创作。
        </p>
        <p>
          Symi Editor 是基于 Tauri 的跨平台桌面应用，用于编写、编辑和预览 Symi
          代码，支持实时语法高亮、错误检查、实时回放和钢琴窗预览。
        </p>

        <div class="links-row">
          <button
            class="icon-link"
            type="button"
            @click="void openUrl(repoUrl)"
            title="GitHub"
            aria-label="GitHub"
          >
            <Icon name="mdi-github" class="w-5 h-5" />
            <span>GitHub</span>
          </button>
          <button
            class="icon-link"
            type="button"
            @click="void openUrl(discordUrl)"
            title="Discord"
            aria-label="Discord"
          >
            <Icon name="mdi-discord" class="w-5 h-5" />
            <span>Discord</span>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style lang="css" scoped>
.modal-mask {
  @apply fixed inset-0 z-60 bg-black/40 backdrop-blur-[1px] flex items-center justify-center p-4;
}

.modal-panel {
  @apply w-full max-w-2xl rounded-lg border border-slate-700 bg-slate-900 text-slate-200 shadow-xl;
}

.modal-header {
  @apply h-11 px-4 border-b border-slate-700 flex items-center justify-between text-sm font-semibold;
}

.close-btn {
  @apply p-1 rounded text-slate-400 hover:text-slate-200 hover:bg-slate-700 transition-colors;
}

.modal-body {
  @apply p-4 text-sm space-y-3 text-slate-300;

  h3 {
    @apply text-slate-100 font-semibold;
  }

  p {
    @apply leading-6;
  }
}

.hero {
  @apply flex items-center gap-3 pb-1;
}

.logo {
  @apply w-16 h-16 rounded bg-slate-800 p-2 border border-slate-700;
}

.hero-text {
  @apply min-w-0;

  h3 {
    @apply text-base;
  }

  p {
    @apply text-slate-400 leading-5;
  }
}

.meta-row {
  @apply mt-2 flex gap-2;
}

.meta-chip {
  @apply px-2 py-0.5 rounded text-xs bg-slate-800 border border-slate-700 text-slate-200;
}

.links-row {
  @apply pt-1 flex gap-2;
}

.icon-link {
  @apply inline-flex items-center gap-1.5 px-3 py-1.5 rounded text-xs;
  @apply bg-slate-700 text-slate-100 hover:bg-slate-600 transition-colors;

  span {
    @apply leading-none;
  }
}
</style>
