<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { useLocalStorage } from "@vueuse/core";
import { computed, onMounted } from "vue";
import { cursorInfo } from "../utils/cm";

const barText = computed(() => {
  return cursorInfo.bar == null ? "-" : `${cursorInfo.bar}`;
});

const timeText = computed(() => {
  if (cursorInfo.seconds == null) return "-";
  let total = Math.round(Math.max(0, cursorInfo.seconds) * 1000) / 1000;
  let min = Math.floor(total / 60);
  let sec = Math.floor(total % 60);
  let ms = Math.round((total - Math.floor(total)) * 1000);
  const secStr = sec.toString().padStart(2, "0");
  const msStr = ms.toString().padStart(3, "0");
  return `${min}:${secStr}.${msStr}`;
});

const volume = useLocalStorage<number>("symi:volume", 50);

const volumeText = computed(() => `${volume.value}%`);

function clampVolume(value: number): number {
  return Math.round(Math.max(0, Math.min(100, value)));
}

async function syncVolumeToBackend() {
  try {
    const actual = await invoke<number>("set_volume", {
      volume: clampVolume(volume.value) / 100,
    });
    volume.value = clampVolume(actual * 100);
  } catch (error) {
    console.error("[StatusBar] set_volume failed", error);
  }
}

async function handleVolumeInput(event: Event) {
  const target = event.target as HTMLInputElement | null;
  if (!target) return;
  const next = Number(target.value);
  if (Number.isFinite(next)) {
    volume.value = clampVolume(next);
    await syncVolumeToBackend();
  }
}

onMounted(() => {
  volume.value = clampVolume(volume.value);
  void syncVolumeToBackend();
});
</script>

<template>
  <div class="status-bar">
    <div class="status-item">
      行:列
      <span class="status-value"
        >{{ cursorInfo.line }}:{{ cursorInfo.column }}</span
      >
    </div>
    <div class="status-item">
      小节
      <span class="status-value grid grid-cols-[auto_auto] items-center gap-1">
        <span class="text-end">{{ barText }}</span>
        <div class="flex flex-col items-center w-fit">
          <span class="text-xs flex-1">{{
            cursorInfo.tick?.[0] ?? "-"
          }}</span>
          <div class="h-px bg-slate-500 my-0.5 w-full"></div>
          <span class="text-xs flex-1">{{
            cursorInfo.tick?.[1] ?? "-"
          }}</span>
        </div>
      </span>
    </div>
    <div class="status-item">
      时间 <span class="status-value">{{ timeText }}</span>
    </div>
    <div class="status-item status-item-volume">
      音量
      <input
        class="volume-slider"
        type="range"
        min="0"
        max="100"
        step="1"
        :value="volume"
        @input="handleVolumeInput"
      />
      <span class="status-value volume-value">{{ volumeText }}</span>
    </div>
  </div>
</template>

<style lang="css" scoped>
.status-bar {
  @apply px-4 py-2 text-sm;
  @apply grid grid-cols-[auto_auto_auto_1fr] gap-8 items-center;
  @apply text-slate-300 bg-slate-900;
}

.status-item {
  @apply inline-flex gap-4 items-center whitespace-nowrap;
}

.status-value {
  @apply text-slate-200 font-semibold;
}

.status-item-volume {
  @apply justify-self-end;
}

.volume-slider {
  @apply w-32 accent-slate-300;
}

.volume-value {
  @apply w-10 text-right;
}
</style>
