<script setup lang="ts">
const props = withDefaults(
  defineProps<{
    modelValue: boolean;
    message: string;
    type?: "success" | "error" | "info";
    duration?: number;
  }>(),
  {
    type: "info",
    duration: 2400,
  },
);

const emit = defineEmits<{
  (e: "update:modelValue", value: boolean): void;
}>();

let timer: ReturnType<typeof setTimeout> | null = null;

function clearTimer() {
  if (!timer) return;
  clearTimeout(timer);
  timer = null;
}

function startTimer() {
  clearTimer();
  if (!props.modelValue || props.duration <= 0) return;
  timer = setTimeout(() => {
    emit("update:modelValue", false);
  }, props.duration);
}

watch(
  () => props.modelValue,
  () => {
    startTimer();
  },
  { immediate: true },
);

watch(
  () => props.message,
  () => {
    startTimer();
  },
);

onBeforeUnmount(() => {
  clearTimer();
});
</script>

<template>
  <Transition name="toast-fade">
    <div v-if="modelValue" class="toast-wrap">
      <div class="toast" :class="[`toast-${type}`]">
        {{ message }}
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.toast-wrap {
  @apply fixed z-[70] right-4 top-4;
}

.toast {
  @apply px-3 py-2 rounded border text-sm;
  @apply bg-slate-900 text-slate-200 border-slate-700;
}

.toast-success {
  @apply border-emerald-500/60 text-emerald-300;
}

.toast-error {
  @apply border-red-500/60 text-red-300;
}

.toast-info {
  @apply border-slate-600 text-slate-200;
}

.toast-fade-enter-active,
.toast-fade-leave-active {
  transition: opacity 0.2s ease;
}

.toast-fade-enter-from,
.toast-fade-leave-to {
  opacity: 0;
}
</style>
