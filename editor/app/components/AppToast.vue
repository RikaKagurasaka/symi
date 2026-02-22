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
    <div v-if="modelValue" class="fixed z-70 right-4 top-4">
      <div
        class="px-3 py-2 rounded border text-sm bg-slate-900"
        :class="[
          type === 'success' ? 'border-emerald-500/60 text-emerald-300' : '',
          type === 'error' ? 'border-red-500/60 text-red-300' : '',
          type === 'info' ? 'border-slate-600 text-slate-200' : ''
        ]"
      >
        {{ message }}
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.toast-fade-enter-active,
.toast-fade-leave-active {
  transition: opacity 0.2s ease;
}

.toast-fade-enter-from,
.toast-fade-leave-to {
  opacity: 0;
}
</style>
