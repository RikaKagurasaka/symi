<script setup lang="ts">
interface ReleaseAsset {
  id: number;
  name: string;
  size: number;
  contentType: string;
  downloadCount: number;
  updatedAt: string;
  browserDownloadUrl: string;
  proxyDownloadUrl: string;
}

interface LatestReleasePayload {
  repo: string;
  id: number;
  tag: string;
  name: string;
  publishedAt: string | null;
  htmlUrl: string;
  notes: string | null;
  assets: ReleaseAsset[];
}

type DistributionMeta = {
  os: "Linux" | "macOS" | "Windows";
  format:
    | "rpm (Fedora/CentOS)"
    | "deb (Ubuntu/Debian)"
    | "AppImage"
    | "DMG (Apple Silicon)"
    | "tar.gz (Apple Silicon)"
    | "Installer (x64)"
    | "MSI Installer (x64)"
    | "Portable (x64)";
  order: 0 | 1 | 2;
};

function filenameToDistributionType(filename: string) {
  const map = {
    "x86_64.rpm": ["Linux", "rpm (Fedora/CentOS)", 1],
    "amd64.deb": ["Linux", "deb (Ubuntu/Debian)", 0],
    "amd64.AppImage": ["Linux", "AppImage", 2],
    "aarch64.dmg": ["macOS", "DMG (Apple Silicon)", 0],
    "aarch64.app.tar.gz": ["macOS", "tar.gz (Apple Silicon)", 1],
    "x64-setup.exe": ["Windows", "Installer (x64)", 0],
    ".msi": ["Windows", "MSI Installer (x64)", 1],
    "x64-portable.exe": ["Windows", "Portable (x64)", 2],
  } as const;

  const matched = Object.entries(map).find(([suffix]) =>
    filename.endsWith(suffix),
  );
  if (!matched) {
    return null;
  }

  const [, [os, format, order]] = matched;
  return { os, format, order };
}

const { data, status, error, refresh } = await useFetch<LatestReleasePayload>(
  "/api/releases/latest",
  {
    key: "latest-release",
  },
);

type DistributionAsset = {
  asset: ReleaseAsset;
  meta: DistributionMeta;
};

type SystemOption = DistributionMeta["os"];
type FormatOption = DistributionMeta["format"];

const systemOrder: SystemOption[] = ["Windows", "macOS", "Linux"];

function detectSystemFromUserAgent(
  userAgent: string,
): SystemOption | undefined {
  const ua = userAgent.toLowerCase();

  if (ua.includes("windows")) {
    return "Windows";
  }

  if (
    ua.includes("macintosh") ||
    ua.includes("mac os") ||
    ua.includes("darwin")
  ) {
    return "macOS";
  }

  if (ua.includes("linux")) {
    return "Linux";
  }

  return undefined;
}

const preferredSystem = ref<SystemOption | undefined>(undefined);

if (import.meta.server) {
  const headers = useRequestHeaders(["user-agent"]);
  preferredSystem.value = detectSystemFromUserAgent(
    headers["user-agent"] || "",
  );
}

if (import.meta.client) {
  preferredSystem.value = detectSystemFromUserAgent(navigator.userAgent || "");
}

const distributionAssets = computed<DistributionAsset[]>(() => {
  if (!data.value) {
    return [];
  }

  return data.value.assets.reduce<DistributionAsset[]>((result, asset) => {
    const meta = filenameToDistributionType(asset.name);
    if (!meta) {
      return result;
    }

    result.push({ asset, meta });
    return result;
  }, []);
});

const systemOptions = computed<SystemOption[]>(() => {
  const systems = new Set(distributionAssets.value.map((item) => item.meta.os));
  return systemOrder.filter((system) => systems.has(system));
});

const selectedSystem = ref<SystemOption | undefined>(undefined);
const selectedFormat = ref<FormatOption | undefined>(undefined);

const currentSystemAssets = computed(() => {
  if (!selectedSystem.value) {
    return [];
  }

  return distributionAssets.value.filter(
    (item) => item.meta.os === selectedSystem.value,
  );
});

const formatOptions = computed<FormatOption[]>(() => {
  const orderMap = new Map<FormatOption, number>();
  for (const item of currentSystemAssets.value) {
    const currentOrder = orderMap.get(item.meta.format);
    if (currentOrder === undefined || item.meta.order < currentOrder) {
      orderMap.set(item.meta.format, item.meta.order);
    }
  }

  return [...orderMap.entries()]
    .sort((a, b) => a[1] - b[1] || a[0].localeCompare(b[0]))
    .map(([format]) => format);
});

const selectedDistribution = computed<DistributionAsset | null>(() => {
  if (!selectedSystem.value || !selectedFormat.value) {
    return null;
  }

  const candidates = currentSystemAssets.value
    .filter((item) => item.meta.format === selectedFormat.value)
    .sort(
      (a, b) =>
        a.meta.order - b.meta.order || a.asset.name.localeCompare(b.asset.name),
    );

  return candidates[0] || null;
});

watch(
  systemOptions,
  (options) => {
    if (!options.length) {
      selectedSystem.value = undefined;
      return;
    }

    if (!selectedSystem.value || !options.includes(selectedSystem.value)) {
      if (preferredSystem.value && options.includes(preferredSystem.value)) {
        selectedSystem.value = preferredSystem.value;
        return;
      }

      selectedSystem.value = options[0];
    }
  },
  { immediate: true },
);

watch(
  formatOptions,
  (options) => {
    if (!options.length) {
      selectedFormat.value = undefined;
      return;
    }

    if (!selectedFormat.value || !options.includes(selectedFormat.value)) {
      selectedFormat.value = options[0];
    }
  },
  { immediate: true },
);

function formatBytes(value: number) {
  if (value <= 0) {
    return "0 B";
  }

  const units = ["B", "KB", "MB", "GB"];
  const index = Math.min(
    Math.floor(Math.log(value) / Math.log(1024)),
    units.length - 1,
  );
  const size = value / 1024 ** index;

  return `${size.toFixed(index === 0 ? 0 : 1)} ${units[index]}`;
}

function formatDate(value: string | null) {
  if (!value) {
    return "未知";
  }

  return new Date(value).toLocaleString("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function openLink(url: string) {
  window.open(url, "_blank");
}
</script>

<template>
  <div class="space-y-4">
    <UAlert
      v-if="status === 'pending'"
      color="neutral"
      variant="subtle"
      title="正在获取最新版本信息..."
      icon="i-lucide-loader-circle"
    />

    <UAlert
      v-else-if="error"
      color="error"
      variant="subtle"
      title="获取最新版本失败"
      :description="error.message"
      icon="i-lucide-triangle-alert"
    >
      <template #actions>
        <UButton
          size="xs"
          color="error"
          variant="outline"
          label="重试"
          @click="() => refresh()"
        />
      </template>
    </UAlert>

    <div v-else-if="data" class="space-y-4">
      <UCard>
        <template #header>
          <div class="flex flex-wrap items-center justify-between gap-2">
            <div>
              <p class="font-medium">{{ data.name }}</p>
              <p class="text-sm text-muted">
                Tag: {{ data.tag }} · 发布时间：{{
                  formatDate(data.publishedAt)
                }}
              </p>
            </div>
            <UButton
              label="Github Release"
              icon="i-lucide-external-link"
              color="neutral"
              variant="outline"
              :to="data.htmlUrl"
              target="_blank"
            />
          </div>
        </template>

        <div class="space-y-3">
          <div class="grid gap-3 md:grid-cols-2">
            <UFormField label="系统">
              <USelect
                class="w-full"
                v-model="selectedSystem"
                :items="systemOptions"
                placeholder="请选择系统"
                :disabled="!systemOptions.length"
              />
            </UFormField>

            <UFormField label="格式">
              <USelect
                class="w-full"
                v-model="selectedFormat"
                :items="formatOptions"
                placeholder="请选择格式"
                :disabled="!formatOptions.length"
              />
            </UFormField>
          </div>

          <div
            v-if="selectedDistribution"
            class="flex flex-wrap items-center justify-between gap-3 rounded-md border border-default p-3"
          >
            <div class="min-w-0 flex-1">
              <p class="truncate font-mono text-sm">
                {{ selectedDistribution.asset.name }}
              </p>
              <p class="text-xs text-muted">
                {{ formatBytes(selectedDistribution.asset.size) }}
              </p>
            </div>

            <UButton
              label="下载"
              icon="i-lucide-download"
              color="primary"
              @click="() => openLink(selectedDistribution!.asset.proxyDownloadUrl)"
            />
          </div>
          <UAlert
            v-else
            color="warning"
            variant="subtle"
            title="当前版本没有可识别的分发包"
            description="请到 GitHub Release 页面查看完整资产列表。"
            icon="i-lucide-circle-alert"
          />

          <UAlert
            v-if="selectedFormat === 'Portable (x64)'"
            color="warning"
            variant="subtle"
            title="不建议使用Portable版本"
            description="其无法检测缺少的依赖，可能导致无法启动等问题。除非你能够自己解决Tauri依赖，否则请优先选择Installer版本。"
            icon="i-lucide-triangle-alert"
          />
        </div>
      </UCard>
    </div>
  </div>
</template>
