<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { useDebounceFn, useLocalStorage } from "@vueuse/core";

type MidiExportPrefs = {
  targetPath: string;
  pitchBendRangeSemitones: number;
  ticksPerQuarter: number;
  timeToleranceSeconds: number;
  pitchToleranceCents: number;
};

const props = defineProps<{
  modelValue: boolean;
  fileId: string | null;
  source: string;
  defaultName: string;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: boolean): void;
  (e: "export-result", payload: { ok: boolean; message: string }): void;
}>();

const prefs = useLocalStorage<MidiExportPrefs>("symi:midi-export-prefs", {
  targetPath: "",
  pitchBendRangeSemitones: 2,
  ticksPerQuarter: 480,
  timeToleranceSeconds: 0.001,
  pitchToleranceCents: 5,
});

const validationError = ref<string>("");
const isValidating = ref(false);
const isExporting = ref(false);

const canExport = computed(() => {
  return (
    !!props.fileId &&
    !!props.source &&
    !!prefs.value.targetPath &&
    !validationError.value &&
    !isValidating.value &&
    !isExporting.value
  );
});

function closeModal() {
  emit("update:modelValue", false);
}

function normalizeMidiPath(path: string) {
  const lower = path.toLowerCase();
  if (lower.endsWith(".mid") || lower.endsWith(".midi")) return path;
  return path.replace(/\.[^./\\]+$/, "") + ".mid";
}

function buildDefaultPath() {
  const fallback = props.defaultName || "Untitled";
  return normalizeMidiPath(fallback);
}

async function pickPath() {
  const selected = await save({
    defaultPath: prefs.value.targetPath || buildDefaultPath(),
    filters: [{ name: "MIDI Files", extensions: ["mid", "midi"] }],
  });
  if (!selected) return;
  prefs.value.targetPath = normalizeMidiPath(selected);
}

function validateLocalParams(): string {
  if (!props.fileId) return "当前没有可导出的文件";
  if (!props.source) return "当前文件内容为空";
  if (!prefs.value.targetPath) return "请先选择导出路径";
  if (prefs.value.pitchBendRangeSemitones <= 0) {
    return "弯音最大半音数必须大于 0";
  }
  if (prefs.value.ticksPerQuarter <= 0) {
    return "MIDI 分辨率必须大于 0";
  }
  if (prefs.value.timeToleranceSeconds < 0) {
    return "时间容差不能小于 0";
  }
  if (prefs.value.pitchToleranceCents < 0) {
    return "音高容差不能小于 0";
  }
  return "";
}

async function runValidation() {
  const localError = validateLocalParams();
  if (localError) {
    validationError.value = localError;
    return;
  }

  isValidating.value = true;
  validationError.value = "";
  try {
    await invoke("validate_midi_export", {
      fileId: props.fileId,
      source: props.source,
      pitchBendRangeSemitones: Math.round(prefs.value.pitchBendRangeSemitones),
      ticksPerQuarter: Math.round(prefs.value.ticksPerQuarter),
      timeToleranceSeconds: prefs.value.timeToleranceSeconds,
      pitchToleranceCents: prefs.value.pitchToleranceCents,
    });
  } catch (error) {
    validationError.value = String(error);
  } finally {
    isValidating.value = false;
  }
}

const runValidationDebounced = useDebounceFn(() => {
  void runValidation();
}, 250);

watch(
  () => props.modelValue,
  (open) => {
    if (!open) return;
    if (!prefs.value.targetPath) {
      prefs.value.targetPath = buildDefaultPath();
    }
    runValidationDebounced();
  },
);

watch(
  () => [
    props.fileId,
    props.source,
    prefs.value.targetPath,
    prefs.value.pitchBendRangeSemitones,
    prefs.value.ticksPerQuarter,
    prefs.value.timeToleranceSeconds,
    prefs.value.pitchToleranceCents,
    props.modelValue,
  ],
  (values) => {
    const open = values[7];
    if (!open) return;
    runValidationDebounced();
  },
);

async function exportMidi() {
  if (!canExport.value) return;
  isExporting.value = true;
  validationError.value = "";
  try {
    await invoke("export_midi", {
      fileId: props.fileId,
      source: props.source,
      targetPath: prefs.value.targetPath,
      pitchBendRangeSemitones: Math.round(prefs.value.pitchBendRangeSemitones),
      ticksPerQuarter: Math.round(prefs.value.ticksPerQuarter),
      timeToleranceSeconds: prefs.value.timeToleranceSeconds,
      pitchToleranceCents: prefs.value.pitchToleranceCents,
    });
    emit("export-result", { ok: true, message: "MIDI 导出成功" });
    closeModal();
  } catch (error) {
    const message = String(error);
    validationError.value = message;
    emit("export-result", { ok: false, message });
  } finally {
    isExporting.value = false;
  }
}
</script>

<template>
  <div v-if="modelValue" class="modal-mask" @click.self="closeModal">
    <div class="modal-panel">
      <div class="modal-header">导出 MIDI</div>

      <div class="field-grid">
        <label class="field-label">导出路径</label>
        <div class="field-path-row">
          <input v-model="prefs.targetPath" class="field-input" type="text" />
          <button class="action-btn" @click="pickPath">选择</button>
        </div>

        <label class="field-label">弯音半音数(RPN)</label>
        <input
          v-model.number="prefs.pitchBendRangeSemitones"
          class="field-input"
          type="number"
          min="1"
          step="1"
        />

        <label class="field-label">MIDI分辨率(TPQ)</label>
        <input
          v-model.number="prefs.ticksPerQuarter"
          class="field-input"
          type="number"
          min="1"
          step="1"
        />

        <label class="field-label">时间容差(秒)</label>
        <input
          v-model.number="prefs.timeToleranceSeconds"
          class="field-input"
          type="number"
          min="0"
          step="0.0001"
        />

        <label class="field-label">音高容差(音分)</label>
        <input
          v-model.number="prefs.pitchToleranceCents"
          class="field-input"
          type="number"
          min="0"
          step="0.1"
        />
      </div>

      <div class="status-row">
        <span v-if="isValidating" class="status-text">正在校验导出参数...</span>
        <span v-else-if="validationError" class="status-error">{{ validationError }}</span>
        <span v-else class="status-ok">参数校验通过，可导出</span>
      </div>

      <div class="footer-row">
        <button class="action-btn" @click="closeModal">取消</button>
        <button class="action-btn export-btn" :disabled="!canExport" @click="exportMidi">
          {{ isExporting ? "导出中..." : "导出" }}
        </button>
      </div>
    </div>
  </div>
</template>

<style lang="css" scoped>
.modal-mask {
  @apply fixed inset-0 z-50 flex items-center justify-center;
  @apply bg-slate-900/70;
}

.modal-panel {
  @apply w-[640px] max-w-[92vw] rounded border p-4;
  @apply bg-slate-800 border-slate-700 text-slate-200;
}

.modal-header {
  @apply text-base font-semibold mb-4;
}

.field-grid {
  @apply grid grid-cols-[140px_1fr] gap-x-3 gap-y-3 items-center;
}

.field-label {
  @apply text-sm text-slate-300;
}

.field-path-row {
  @apply grid grid-cols-[1fr_auto] gap-2;
}

.field-input {
  @apply h-8 rounded border px-2 text-sm;
  @apply bg-slate-900 border-slate-700 text-slate-200;
}

.action-btn {
  @apply h-8 rounded px-3 text-sm transition-colors duration-150 ease-in-out cursor-pointer;
  @apply bg-slate-700 text-slate-200 hover:bg-slate-600;
}

.status-row {
  @apply mt-4 min-h-5 text-sm;
}

.status-text {
  @apply text-slate-300;
}

.status-ok {
  @apply text-emerald-400;
}

.status-error {
  @apply text-red-400;
}

.footer-row {
  @apply mt-4 flex justify-end gap-2;
}

.export-btn:disabled {
  @apply opacity-50 cursor-not-allowed hover:bg-slate-700;
}
</style>
