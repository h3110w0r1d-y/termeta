<script setup lang="ts">
import { computed } from "vue";
import { NButton, NEmpty, NIcon, NPopover, NProgress, NText } from "naive-ui";
import { IconArrowsTransferUpDown, IconX } from "@tabler/icons-vue";
import { t, type PlainMessageKey } from "@/i18n";
import { useTerminalSurfaceStyle } from "../terminal/useTerminalSurface";
import { formatFileSize } from "./format";
import {
  transferPercentage,
  type SftpTransferActivity,
} from "./transferActivity";

const surfaceStyle = useTerminalSurfaceStyle();
const props = defineProps<{ tasks: readonly SftpTransferActivity[] }>();
const emit = defineEmits<{ cancel: [id: string]; clearFinished: [] }>();
const active = computed(() =>
  props.tasks.filter(
    (task) => task.status === "running" || task.status === "queued",
  ),
);
const recent = computed(() => [...props.tasks].reverse());
const current = computed(
  () =>
    active.value.find((task) => task.status === "running") ?? active.value[0],
);
const finished = computed(() => props.tasks.length > active.value.length);
const statusKeys: Record<SftpTransferActivity["status"], PlainMessageKey> = {
  queued: "common.status.waiting",
  running: "files.transfer.inProgress",
  done: "files.transfer.complete",
  cancelled: "common.status.cancelled",
  failed: "files.activity.failed",
};
function status(task: SftpTransferActivity) {
  if (task.cancelling) return t("files.activity.cancelling");
  return task.message ?? t(task.messageKey ?? statusKeys[task.status]);
}
function detail(task: SftpTransferActivity) {
  const progress = task.progress;
  if (!progress) return status(task);
  return t("files.transfer.summary", {
    name: progress.currentName ?? task.label,
    completed: progress.completedFiles ?? 0,
    total: progress.totalFiles ?? "?",
    transferred: formatFileSize(progress.transferred),
    size: progress.total !== null ? ` / ${formatFileSize(progress.total)}` : "",
  });
}
function progressStatus(task: SftpTransferActivity) {
  if (task.status === "failed") return "error" as const;
  if (task.status === "done") return "success" as const;
  if (task.status === "cancelled") return "warning" as const;
  return "default" as const;
}
</script>

<template>
  <footer class="transfer-footer" :aria-label="t('files.activity.title')">
    <NPopover
      trigger="click"
      placement="top-start"
      :show-arrow="false"
      :width="420"
      :style="[surfaceStyle, { maxWidth: 'calc(100vw - 32px)' }]"
      :content-style="{ padding: '0' }"
    >
      <template #trigger>
        <NButton
          quaternary
          size="small"
          class="activity-toggle"
          :title="t('files.activity.open')"
          :aria-label="t('files.activity.open')"
        >
          <template #icon>
            <NIcon aria-hidden="true"><IconArrowsTransferUpDown /></NIcon>
          </template>
          {{
            active.length
              ? t("files.activity.active", { count: active.length })
              : t("files.activity.title")
          }}
          <span v-if="tasks.length" class="activity-count">
            {{ tasks.length }}
          </span>
        </NButton>
      </template>
      <section class="activity-panel" :aria-label="t('files.activity.title')">
        <header class="activity-header">
          <strong>{{ t("files.activity.title") }}</strong>
          <NButton
            size="tiny"
            quaternary
            :disabled="!finished"
            @click="emit('clearFinished')"
          >
            {{ t("files.activity.clearFinished") }}
          </NButton>
        </header>
        <NEmpty
          v-if="!tasks.length"
          class="activity-empty"
          size="small"
          :description="t('files.activity.empty')"
        />
        <ol v-else class="activity-list">
          <li v-for="task in recent" :key="task.id" class="activity-task">
            <div class="task-heading">
              <span class="task-name" :title="task.label">
                {{ task.label }}
              </span>
              <NButton
                v-if="task.status === 'queued' || task.status === 'running'"
                quaternary
                circle
                size="tiny"
                :disabled="task.cancelling"
                :aria-label="t('files.actions.cancelTransfer')"
                :title="t('files.actions.cancelTransfer')"
                @click="emit('cancel', task.id)"
              >
                <NIcon :size="15" aria-hidden="true"><IconX /></NIcon>
              </NButton>
            </div>
            <NText
              v-if="task.path"
              class="task-path"
              depth="3"
              :title="task.path"
            >
              {{ task.path }}
            </NText>
            <div
              class="progress-track"
              :class="{
                'is-indeterminate':
                  task.status === 'running' &&
                  transferPercentage(task) === null,
              }"
            >
              <NProgress
                type="line"
                :height="5"
                :percentage="transferPercentage(task) ?? 0"
                :status="progressStatus(task)"
                :processing="task.status === 'running'"
                :show-indicator="false"
                :aria-label="t('files.activity.progress')"
                :aria-valuenow="transferPercentage(task) ?? undefined"
                :aria-valuetext="
                  transferPercentage(task) === null
                    ? t('files.activity.unknownProgress')
                    : undefined
                "
              />
            </div>
            <div class="task-detail">
              <NText
                :type="task.status === 'failed' ? 'error' : undefined"
                depth="3"
              >
                {{ status(task) }}
              </NText>
              <NText v-if="transferPercentage(task) !== null" depth="3">
                {{ Math.floor(transferPercentage(task)!) }}%
              </NText>
            </div>
            <NText v-if="task.progress" class="task-description" depth="3">
              {{ detail(task) }}
            </NText>
          </li>
        </ol>
      </section>
    </NPopover>
    <div v-if="current" class="footer-progress">
      <div
        class="progress-track"
        :class="{
          'is-indeterminate':
            current.status === 'running' &&
            transferPercentage(current) === null,
        }"
      >
        <NProgress
          type="line"
          :height="4"
          :percentage="transferPercentage(current) ?? 0"
          :processing="current.status === 'running'"
          :show-indicator="false"
          :aria-label="t('files.activity.progress')"
          :aria-valuenow="transferPercentage(current) ?? undefined"
          :aria-valuetext="
            transferPercentage(current) === null
              ? t('files.activity.unknownProgress')
              : undefined
          "
        />
      </div>
      <NText class="footer-detail" depth="3" :title="detail(current)">
        {{ detail(current) }}
      </NText>
    </div>
    <NText v-else class="footer-idle" depth="3">
      {{ tasks.length ? t("files.activity.idle") : t("files.activity.empty") }}
    </NText>
  </footer>
</template>

<style scoped>
.transfer-footer {
  display: flex;
  align-items: center;
  flex: none;
  gap: 12px;
  min-width: 0;
  min-height: 42px;
  padding: 5px 10px;
  border-top: 1px solid var(--border);
  background: var(--surface);
}
.activity-toggle {
  flex: none;
}
.activity-count {
  margin-inline-start: 8px;
  padding-inline: 6px;
  border-radius: 9px;
  background: var(--surface-hover);
  font-size: 11px;
  font-variant-numeric: tabular-nums;
}
.footer-progress {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 3px;
  min-width: 0;
  max-width: 480px;
}
.progress-track {
  position: relative;
  overflow: hidden;
  border-radius: 3px;
}
.progress-track.is-indeterminate::after {
  position: absolute;
  inset: 0 auto 0 0;
  width: 32%;
  border-radius: inherit;
  background: var(--accent);
  content: "";
  animation: activity-indeterminate 1.5s ease-in-out infinite;
  pointer-events: none;
}
@keyframes activity-indeterminate {
  from {
    transform: translateX(-110%);
  }
  to {
    transform: translateX(420%);
  }
}
@media (prefers-reduced-motion: reduce) {
  .progress-track.is-indeterminate::after {
    animation: none;
    inset-inline-start: 34%;
    opacity: 0.65;
  }
}
.footer-detail,
.footer-idle {
  overflow: hidden;
  font-size: 11px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.activity-panel {
  min-width: 0;
}
.activity-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 14px;
  border-bottom: 1px solid var(--border);
}
.activity-empty {
  padding: 30px 16px;
}
.activity-list {
  max-height: min(420px, 60vh);
  margin: 0;
  padding: 0;
  overflow-y: auto;
  list-style: none;
}
.activity-task {
  display: flex;
  flex-direction: column;
  gap: 7px;
  padding: 12px 14px;
}
.activity-task + .activity-task {
  border-top: 1px solid var(--border);
}
.task-heading,
.task-detail {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  min-width: 0;
}
.task-name,
.task-path {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.task-name {
  font-weight: 500;
}
.task-path,
.task-detail,
.task-description {
  font-size: 12px;
}
.task-detail > :first-child,
.task-description {
  overflow-wrap: anywhere;
}
</style>
