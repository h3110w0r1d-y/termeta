<script setup lang="ts">
import { computed, h, reactive, ref, watch } from "vue";
import {
  NAlert,
  NButton,
  NCheckbox,
  NDropdown,
  NForm,
  NFormItem,
  NInput,
  type FormInst,
  type FormItemInst,
} from "naive-ui";
import { IconLock, IconTrash } from "@tabler/icons-vue";
import AppDialog from "@/components/ui/AppDialog.vue";
import { closeContextMenuOnOutside } from "@/components/ui/contextMenu";
import { t, type PlainMessageKey } from "@/i18n";
import { useTerminalSurfaceStyle } from "../terminal/useTerminalSurface";
import { desktop, errorMessage } from "@/services/desktop/client";
import type { FileEntry, SftpEntryRequest } from "@/services/desktop/types";
import { validateForm } from "@/services/forms/rules";

const surfaceStyle = useTerminalSurfaceStyle();
const props = defineProps<{
  sessionId?: string;
  path: string;
  entries: FileEntry[];
  disabled?: boolean;
}>();
const emit = defineEmits<{
  changed: [];
  busy: [value: boolean];
}>();
const menu = reactive({ show: false, x: 0, y: 0 });
const selected = ref<FileEntry[]>([]);
const context = ref<SftpEntryRequest>();
const dialog = ref<"delete" | "permissions" | null>(null);
const recursiveConfirmation = ref(false);
const recursiveAccepted = ref(false);
const busy = ref(false);
const error = ref("");
const form = ref<FormInst>();
const modeItem = ref<FormItemInst>();
const model = reactive({ mode: "" });
const mode = computed(() =>
  /^[0-7]{3,4}$/.test(model.mode) ? Number.parseInt(model.mode, 8) : null,
);
const rules = computed(() => ({
  mode: {
    required: true,
    trigger: ["input", "change", "blur"],
    validator: () =>
      mode.value !== null || new Error(t("files.manage.permissions.invalid")),
  },
}));
const hasDirectory = computed(() =>
  selected.value.some((entry) => entry.kind === "directory"),
);
const editable = computed(
  () =>
    selected.value.length > 0 &&
    selected.value.every(
      (entry) => entry.kind === "file" || entry.kind === "directory",
    ),
);
const options = computed(() => [
  {
    key: "permissions",
    label: t("files.manage.permissions.title"),
    icon: () => h(IconLock, { size: 16, "aria-hidden": true }),
    disabled: !editable.value || !!props.disabled,
  },
  { type: "divider", key: "divider" },
  {
    key: "delete",
    label: t("files.manage.delete.action"),
    icon: () => h(IconTrash, { size: 16, "aria-hidden": true }),
    disabled: !!props.disabled,
  },
]);
const subjects: { key: PlainMessageKey; shift: number }[] = [
  { key: "files.manage.permissions.owner", shift: 6 },
  { key: "files.manage.permissions.group", shift: 3 },
  { key: "files.manage.permissions.others", shift: 0 },
];
const rights: { key: PlainMessageKey; bit: number }[] = [
  { key: "files.manage.permissions.read", bit: 4 },
  { key: "files.manage.permissions.write", bit: 2 },
  { key: "files.manage.permissions.execute", bit: 1 },
];
const special: { key: PlainMessageKey; bit: number }[] = [
  { key: "files.manage.permissions.setuid", bit: 0o4000 },
  { key: "files.manage.permissions.setgid", bit: 0o2000 },
  { key: "files.manage.permissions.sticky", bit: 0o1000 },
];
function matchesContext() {
  return (
    context.value?.sessionId === props.sessionId &&
    context.value?.path === props.path
  );
}
function open(event: MouseEvent, row: FileEntry) {
  event.preventDefault();
  if (props.disabled || busy.value || !props.sessionId || row.name === "..")
    return;
  selected.value = (
    props.entries.some((entry) => entry.name === row.name)
      ? props.entries
      : [row]
  )
    .filter((entry) => entry.name !== "..")
    .map((entry) => ({ ...entry }));
  context.value = {
    sessionId: props.sessionId,
    path: props.path,
    names: selected.value.map((entry) => entry.name),
  };
  menu.x = event.clientX;
  menu.y = event.clientY;
  menu.show = true;
}
function selectAction(key: string) {
  menu.show = false;
  if (props.disabled || busy.value || !matchesContext()) return;
  error.value = "";
  recursiveConfirmation.value = false;
  recursiveAccepted.value = false;
  if (key === "delete") dialog.value = "delete";
  if (key === "permissions" && editable.value) {
    const modes = new Set(
      selected.value.map((entry) =>
        entry.permissions === null ? null : entry.permissions & 0o7777,
      ),
    );
    const initial = modes.size === 1 ? [...modes][0] : null;
    model.mode = initial == null ? "" : initial.toString(8).padStart(3, "0");
    dialog.value = "permissions";
  }
}
function updateBit(bit: number, checked: boolean) {
  const previous = mode.value ?? 0;
  model.mode = (checked ? previous | bit : previous & ~bit)
    .toString(8)
    .padStart(3, "0");
  void modeItem.value?.validate("change").catch(() => undefined);
}
async function apply() {
  if (props.disabled || busy.value || !matchesContext() || !context.value)
    return;
  if (dialog.value === "permissions" && !(await validateForm(form.value)))
    return;
  if (props.disabled || busy.value || !matchesContext() || !context.value)
    return;
  if (
    dialog.value === "delete" &&
    hasDirectory.value &&
    !recursiveConfirmation.value
  ) {
    recursiveConfirmation.value = true;
    return;
  }
  if (recursiveConfirmation.value && !recursiveAccepted.value) return;
  const request = { ...context.value, names: [...context.value.names] };
  busy.value = true;
  emit("busy", true);
  error.value = "";
  try {
    if (dialog.value === "delete") {
      await desktop.sftpDelete(request, recursiveConfirmation.value);
    } else if (dialog.value === "permissions" && mode.value !== null) {
      await desktop.sftpSetPermissions(request, mode.value);
    }
    if (matchesContext()) dialog.value = null;
  } catch (cause) {
    if (matchesContext()) error.value = errorMessage(cause);
  } finally {
    busy.value = false;
    emit("busy", false);
    // 服务端失败可能发生在部分条目已改变后，始终重新读取当前目录。
    if (request.sessionId === props.sessionId && request.path === props.path)
      emit("changed");
  }
}
watch(
  () => [props.sessionId, props.path],
  () => {
    menu.show = false;
    dialog.value = null;
    context.value = undefined;
    error.value = "";
  },
);
const interacting = computed(() => menu.show || dialog.value !== null);
defineExpose({ open, interacting });
</script>

<template>
  <NDropdown
    :style="surfaceStyle"
    trigger="manual"
    placement="bottom-start"
    :show="menu.show"
    :x="menu.x"
    :y="menu.y"
    :options="options"
    @select="selectAction"
    @clickoutside="closeContextMenuOnOutside($event, menu)"
  />
  <AppDialog
    :style="surfaceStyle"
    :open="dialog !== null"
    :title="
      dialog === 'permissions'
        ? t('files.manage.permissions.title')
        : recursiveConfirmation
          ? t('files.manage.delete.recursiveTitle')
          : t('files.manage.delete.title')
    "
    :locked="busy"
    @update:open="(value) => !value && (dialog = null)"
  >
    <p class="selection-summary">
      {{ t("files.manage.selection", { count: selected.length }) }}
    </p>
    <ul class="selection-names">
      <li
        v-for="entry in selected.slice(0, 100)"
        :key="entry.name"
        :title="entry.name"
      >
        {{ entry.name }}
      </li>
    </ul>
    <NAlert v-if="selected.length > 100" type="warning" class="operation-alert">
      {{ t("files.manage.limit") }}
    </NAlert>
    <NAlert v-if="error" type="error" class="operation-alert">
      {{ error }}
    </NAlert>
    <NAlert v-if="dialog === 'delete'" type="warning">
      {{
        recursiveConfirmation
          ? t("files.manage.delete.recursiveWarning")
          : t("files.manage.delete.warning")
      }}
    </NAlert>
    <NCheckbox
      v-if="dialog === 'delete' && recursiveConfirmation"
      v-model:checked="recursiveAccepted"
      class="recursive-accept"
      :disabled="busy || disabled"
    >
      {{ t("files.manage.delete.confirmContents") }}
    </NCheckbox>
    <template v-if="dialog === 'permissions'">
      <p class="muted">{{ t("files.manage.permissions.applies") }}</p>
      <NForm ref="form" :model="model" :rules="rules" label-placement="top">
        <div class="permission-grid" role="group">
          <span />
          <span v-for="right in rights" :key="right.bit" class="right-label">
            {{ t(right.key) }}
          </span>
          <template v-for="subject in subjects" :key="subject.shift">
            <span class="subject-label">{{ t(subject.key) }}</span>
            <NCheckbox
              v-for="right in rights"
              :key="right.bit"
              class="permission-check"
              :checked="!!((mode ?? 0) & (right.bit << subject.shift))"
              :disabled="busy || disabled"
              :aria-label="`${t(subject.key)} · ${t(right.key)}`"
              @update:checked="updateBit(right.bit << subject.shift, $event)"
            />
          </template>
        </div>
        <NFormItem
          ref="modeItem"
          :label="t('files.manage.permissions.code')"
          path="mode"
        >
          <NInput
            v-model:value="model.mode"
            :disabled="busy || disabled"
            placeholder="755"
            :maxlength="4"
            :input-props="{ inputmode: 'numeric', spellcheck: false }"
            @keydown.enter.prevent="apply"
          />
        </NFormItem>
        <details class="special-permissions">
          <summary>{{ t("files.manage.permissions.special") }}</summary>
          <div class="special-options">
            <NCheckbox
              v-for="item in special"
              :key="item.bit"
              :checked="!!((mode ?? 0) & item.bit)"
              :disabled="busy || disabled"
              @update:checked="updateBit(item.bit, $event)"
            >
              {{ t(item.key) }}
            </NCheckbox>
          </div>
        </details>
      </NForm>
    </template>
    <p v-if="error" class="muted">
      {{ t("files.manage.partialFailure") }}
    </p>
    <template #footer>
      <div class="dialog-actions">
        <NButton :disabled="busy" @click="dialog = null">
          {{ t("common.actions.cancel") }}
        </NButton>
        <NButton
          :type="dialog === 'delete' ? 'error' : 'primary'"
          :loading="busy"
          :disabled="
            disabled ||
            selected.length > 100 ||
            (recursiveConfirmation && !recursiveAccepted)
          "
          @click="apply"
        >
          {{
            dialog === "permissions"
              ? t("common.actions.save")
              : hasDirectory && !recursiveConfirmation
                ? t("files.manage.delete.continue")
                : t("files.manage.delete.permanent")
          }}
        </NButton>
      </div>
    </template>
  </AppDialog>
</template>

<style scoped>
.selection-summary {
  margin: 0 0 8px;
  font-weight: 600;
}
.selection-names {
  max-height: 108px;
  margin: 0 0 16px;
  padding-left: 22px;
  overflow: auto;
}
.selection-names li {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.recursive-accept {
  margin-top: 16px;
}
.operation-alert {
  margin-bottom: 12px;
}
.permission-grid {
  display: grid;
  grid-template-columns: minmax(90px, 1.2fr) repeat(3, minmax(64px, 1fr));
  align-items: center;
  gap: 14px 8px;
  margin: 16px 0 22px;
}
.right-label,
.permission-check {
  justify-self: center;
}
.right-label {
  color: var(--muted);
  text-align: center;
}
.subject-label {
  font-weight: 500;
}
.special-permissions summary {
  cursor: pointer;
  color: var(--muted);
}
.special-options {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 14px 0 4px;
}
.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
</style>
