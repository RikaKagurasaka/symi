<script setup lang="ts">
import noticesText from "../assets/THIRD-PARTY-NOTICES.txt?raw";

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
	<div v-if="modelValue" class="fixed inset-0 z-60 bg-black/40 backdrop-blur-[1px] flex items-center justify-center p-4" @click.self="closeModal">
		<div class="w-full max-w-4xl h-[80vh] rounded-lg border border-slate-700 bg-slate-900 text-slate-200 shadow-xl flex flex-col">
			<div class="h-11 px-4 border-b border-slate-700 flex items-center justify-between text-sm font-semibold shrink-0">
				<span>第三方库许可说明</span>
				<button class="p-1 rounded text-slate-400 hover:text-slate-200 hover:bg-slate-700 transition-colors" @click="closeModal">
					<Icon name="mdi-close" class="w-5 h-5" />
				</button>
			</div>
			<div class="p-4 text-xs text-slate-300 w-full h-full overflow-hidden">
				<textarea class="w-full h-full whitespace-pre-wrap wrap-break-word leading-5" readonly>{{ noticesText }}</textarea>
			</div>
		</div>
	</div>
</template>
