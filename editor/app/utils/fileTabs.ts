import { computed, reactive, ref, toRaw, watch, watchEffect } from "vue";
import { confirm, open, save } from "@tauri-apps/plugin-dialog";
import { readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";
import { invoke } from "@tauri-apps/api/core";
import { useIDBKeyval } from '@vueuse/integrations/useIDBKeyval'
export type FileTab = {
    id: string;
    name: string;
    path?: string;
    content: string;
    isDirty: boolean;
};
type PersistedState = {
    tabs: FileTab[];
    activeFileId: string | null;
    untitledCounter: number;
    idCounter: number;
};

const fileIdb = useIDBKeyval<PersistedState | null>('symi-files', null);
const tabs = reactive<FileTab[]>([]);
const activeFileId = ref<string | null>(null);
let untitledCounter = 1;
let idCounter = 1;
let hydrated = false;

watch(fileIdb.isFinished, () => {
    if (!fileIdb.isFinished.value) return;
    if (hydrated) return;
    const saved = fileIdb.data.value;
    if (!saved) {
        hydrated = true;
        return;
    }
    tabs.splice(0, tabs.length, ...saved.tabs.map((t) => ({ ...t })));
    activeFileId.value = saved.activeFileId ?? (tabs[0]?.id ?? null);
    untitledCounter = saved.untitledCounter ?? 1;
    idCounter = saved.idCounter ?? Math.max(1, tabs.length + 1);
    hydrated = true;

});

watch([tabs, activeFileId], () => {
    if (!hydrated) return;
    const snapshot: PersistedState = {
        tabs: tabs.map((t) => ({ ...toRaw(t) })),
        activeFileId: activeFileId.value,
        untitledCounter,
        idCounter,
    };
    fileIdb.set(snapshot);

}, { deep: true });

function createFileId() {
    return `F${idCounter++}`;
}

function getBaseName(path: string) {
    const parts = path.split(/[/\\]/);
    return parts[parts.length - 1] || path;
}

function setActive(id: string) {
    activeFileId.value = id;
}

function newFile(content = "") {
    const id = createFileId();
    const name = `Untitled ${untitledCounter++}`;
    tabs.push({ id, name, content, isDirty: false });
    setActive(id);
    return id;
}

async function confirmDiscard(tab: FileTab) {
    return confirm(`文件「${tab.name}」尚未保存，确定要关闭吗？`, {
        title: "未保存更改",
        kind: "warning",
    });
}

async function closeFile(id: string) {
    const index = tabs.findIndex((t) => t.id === id);
    if (index === -1) return;
    const tab = tabs[index]!;
    if (tab.isDirty) {
        const ok = await confirmDiscard(tab);
        if (!ok) return;
    }
    tabs.splice(index, 1);
    if (activeFileId.value === id) {
        if (tabs.length > 0) {
            activeFileId.value = tabs[Math.max(0, index - 1)]!.id;
        } else {
            activeFileId.value = null;
        }
    }
    await invoke("file_close", { fileId: id });
}

function updateContent(id: string, content: string) {
    const tab = tabs.find((t) => t.id === id);
    if (!tab) return;
    if (tab.content === content) return;
    tab.content = content;
    tab.isDirty = true;
}

async function openFile() {
    const selected = await open({ multiple: false, filters: [{ name: "Symi Files", extensions: ["symi", "txt"] }] });
    if (!selected || Array.isArray(selected)) return;
    const existing = tabs.find((t) => t.path === selected);
    if (existing) {
        setActive(existing.id);
        return;
    }
    const content = await readTextFile(selected);
    const id = createFileId();
    const name = getBaseName(selected);
    tabs.push({ id, name, path: selected, content, isDirty: false });
    setActive(id);
}

async function saveFile() {
    const tab = tabs.find((t) => t.id === activeFileId.value);
    if (!tab) return;
    if (!tab.path) {
        await saveFileAs();
        return;
    }
    await writeTextFile(tab.path, tab.content);
    tab.isDirty = false;
}

async function saveFileAs() {
    const tab = tabs.find((t) => t.id === activeFileId.value);
    if (!tab) return;
    let defaultPath = tab.path ?? tab.name;
    if (!defaultPath.endsWith(".symi")) {
        defaultPath += ".symi";
    }
    const target = await save({ defaultPath: defaultPath, filters: [{ name: "Symi Files", extensions: ["symi", "txt"] }] });
    if (!target) return;
    await writeTextFile(target, tab.content);
    tab.path = target;
    tab.name = getBaseName(target);
    tab.isDirty = false;
}

const activeTab = computed(() => {
    return tabs.find((t) => t.id === activeFileId.value) ?? null;
});

const hasDirtyTabs = computed(() => tabs.some((t) => t.isDirty));

export function useFileTabs() {
    return {
        tabs,
        activeTab,
        activeFileId,
        setActive,
        newFile,
        updateContent,
        openFile,
        saveFile,
        saveFileAs,
        closeFile,
        hasDirtyTabs,
    };
}
