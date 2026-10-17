import { t, formatDate } from "../../i18n";
import {
  computed,
  onBeforeUnmount,
  onMounted,
  reactive,
  ref,
  watch,
  type Ref,
} from "vue";
import type { FormInst, FormItemInst, FormRules } from "naive-ui";
import { desktop, errorMessage } from "../../services/desktop/client";
import { requiredText, validateForm } from "../../services/forms/rules";
import type {
  BackupConfigDraft,
  BackupConfigView,
  BackupCounts,
  BackupListEntry,
  BackupRestorePreview,
} from "../../services/desktop/types";

export interface BackupPanelOptions {
  initializing: boolean;
  disabled: boolean;
  initialize?: (token: string) => Promise<boolean>;
}

// 云端操作和确认令牌由此管理，界面只保留配置、元数据与计数，不接收备份正文。
export function useCloudBackup(
  props: BackupPanelOptions,
  onBusyChange: (busy: boolean) => void,
  forms: {
    config: Ref<FormInst | undefined>;
    confirmation: Ref<FormItemInst | undefined>;
  },
) {
  const configForm = forms.config;
  const busy = ref(false);
  const error = ref("");
  const success = ref("");
  const saved = ref<BackupConfigView | null>(null);
  const config = reactive<BackupConfigDraft & { confirmation: string }>({
    provider: "gist",
    endpoint: "",
    username: "",
    secret: "",
    backupPassword: "",
    confirmation: "",
    deviceName: t("backup.defaultDeviceName"),
  });
  watch(
    () => config.backupPassword,
    () => {
      if (config.confirmation)
        void forms.confirmation.value
          ?.validate({ trigger: "input" })
          .catch(() => undefined);
    },
  );
  const importWarnings = ref<string[]>([]);
  const action = ref<"backup" | "sync">(props.initializing ? "sync" : "backup");
  const entries = ref<BackupListEntry[]>([]);
  const listed = ref(false);
  const hasMore = ref(false);
  const pageSize = 10;
  let pageOffset = 0;
  const baseline = ref(JSON.stringify(config));
  const preview = ref<BackupRestorePreview>();
  const previewOpen = ref(false);
  const operationBusy = computed(() => busy.value || props.disabled);
  const sameTarget = computed(
    () =>
      !!saved.value &&
      config.provider === saved.value.provider &&
      config.endpoint.trim() === saved.value.endpoint &&
      config.username.trim() === saved.value.username,
  );
  const canReuseSecret = computed(
    () => sameTarget.value && !!saved.value?.hasSecret,
  );
  const canReuseBackupPassword = computed(
    () => sameTarget.value && !!saved.value?.hasBackupPassword,
  );
  const dirty = computed(() => JSON.stringify(config) !== baseline.value);
  const configRules = computed<FormRules>(() => ({
    provider: requiredText("backup.form.provider"),
    deviceName: {
      required: true,
      trigger: ["input", "blur"],
      validator: (_, value: string) =>
        (!!value.trim() &&
          new TextEncoder().encode(value.trim()).length <= 256 &&
          !/[\u0000-\u001f\u007f]/.test(value)) ||
        new Error(t("backup.validation.deviceName")),
    },
    endpoint:
      config.provider === "webdav"
        ? {
            required: true,
            trigger: ["input", "blur"],
            validator: (_, value: string) => {
              try {
                const url = new URL(value);
                return (
                  (url.protocol === "https:" &&
                    !url.username &&
                    !url.password &&
                    !url.search &&
                    !url.hash) ||
                  new Error(t("backup.validation.urlFormat"))
                );
              } catch {
                return new Error(t("backup.validation.webdavUrl"));
              }
            },
          }
        : [],
    username:
      config.provider === "webdav"
        ? {
            required: true,
            trigger: ["input", "blur"],
            validator: (_, value: string) =>
              (!!value.trim() &&
                !value.includes(":") &&
                new TextEncoder().encode(value.trim()).length <= 1024) ||
              new Error(t("backup.validation.username")),
          }
        : [],
    secret: {
      required: !canReuseSecret.value,
      trigger: ["input", "blur"],
      validator: (_, value: string) =>
        !!value ||
        canReuseSecret.value ||
        new Error(t("backup.validation.credentials")),
    },
    backupPassword: {
      required: !canReuseBackupPassword.value,
      trigger: ["input", "blur"],
      validator: (_, value: string) => {
        if (!value)
          return (
            canReuseBackupPassword.value ||
            new Error(t("backup.validation.passwordRequired"))
          );
        if (new TextEncoder().encode(value).length > 1024)
          return new Error(t("backup.validation.passwordMaxLength"));
        if ([...value].length < 12)
          return new Error(t("backup.validation.passwordMinLength"));
        return true;
      },
    },
    confirmation: !!config.backupPassword
      ? {
          required: true,
          trigger: ["input", "blur"],
          validator: (_, value: string) =>
            (value.length > 0 && value === config.backupPassword) ||
            new Error(t("backup.validation.passwordMismatch")),
        }
      : [],
  }));
  function draft(): BackupConfigDraft {
    return {
      provider: config.provider,
      endpoint: config.provider === "webdav" ? config.endpoint.trim() : "",
      username: config.provider === "webdav" ? config.username.trim() : "",
      deviceName: config.deviceName.trim(),
      ...(config.secret ? { secret: config.secret } : {}),
      ...(config.backupPassword
        ? { backupPassword: config.backupPassword }
        : {}),
    };
  }
  function acceptConfig(value: BackupConfigView) {
    saved.value = value;
    config.provider = value.provider;
    config.endpoint = value.endpoint;
    config.username = value.username;
    config.deviceName = value.deviceName;
    config.secret = "";
    config.backupPassword = "";
    config.confirmation = "";
    baseline.value = JSON.stringify(config);
  }
  function invalidate() {
    entries.value = [];
    listed.value = false;
    hasMore.value = false;
    pageOffset = 0;
    previewOpen.value = false;
    preview.value = undefined;
  }
  watch(
    () => [
      config.provider,
      config.endpoint,
      config.username,
      config.secret,
      config.deviceName,
      config.backupPassword,
      action.value,
    ],
    invalidate,
  );
  watch(busy, (value) => onBusyChange(value));
  watch(
    () => config.provider,
    () => {
      config.secret = "";
      config.backupPassword = "";
      config.confirmation = "";
      if (config.provider === "gist") {
        config.endpoint = "";
        config.username = "";
      }
    },
  );
  async function persist() {
    const value = await desktop.saveBackupConfig(draft());
    acceptConfig(value);
  }
  async function save() {
    if (operationBusy.value) return;
    busy.value = true;
    error.value = "";
    success.value = "";
    importWarnings.value = [];
    try {
      if (!(await validateForm(configForm.value))) return;
      await persist();
      success.value = t("backup.status.configSaved");
    } catch (cause) {
      error.value = errorMessage(cause);
    } finally {
      busy.value = false;
    }
  }
  async function testConnection() {
    if (operationBusy.value) return;
    busy.value = true;
    error.value = "";
    success.value = "";
    try {
      if (!(await validateForm(configForm.value))) return;
      await desktop.testBackupConfig(draft(), props.initializing);
      success.value = t("backup.status.connected");
    } catch (cause) {
      error.value = errorMessage(cause);
    } finally {
      busy.value = false;
    }
  }
  async function loadPage() {
    const page = await desktop.listCloudBackups(
      props.initializing ? draft() : undefined,
      props.initializing,
      pageOffset,
      pageSize,
    );
    pageOffset += page.length;
    entries.value = [
      ...new Map(
        [...entries.value, ...page].map((entry) => [entry.id, entry]),
      ).values(),
    ];
    hasMore.value = page.length === pageSize;
    listed.value = true;
  }
  async function loadMore() {
    if (operationBusy.value || !hasMore.value) return;
    busy.value = true;
    error.value = "";
    try {
      await loadPage();
    } catch (cause) {
      error.value = errorMessage(cause);
    } finally {
      busy.value = false;
    }
  }
  async function execute() {
    if (operationBusy.value) return;
    busy.value = true;
    error.value = "";
    success.value = "";
    try {
      if (!(await validateForm(configForm.value))) return;
      if (!props.initializing && (!saved.value || dirty.value)) await persist();
      if (action.value === "backup") {
        const entry = await desktop.createCloudBackup(config.deviceName.trim());
        entries.value = [entry];
        listed.value = true;
        success.value = t("backup.status.uploaded");
        config.backupPassword = "";
        config.confirmation = "";
      } else {
        invalidate();
        await loadPage();
      }
    } catch (cause) {
      error.value = errorMessage(cause);
    } finally {
      busy.value = false;
    }
  }
  async function prepare(entry: BackupListEntry) {
    if (operationBusy.value || !entry.metadata) return;
    busy.value = true;
    error.value = "";
    try {
      if (!(await validateForm(configForm.value))) return;
      preview.value = await desktop.prepareCloudRestore(
        entry.id,
        props.initializing ? draft() : undefined,
        props.initializing,
      );
      previewOpen.value = true;
    } catch (cause) {
      error.value = errorMessage(cause);
    } finally {
      busy.value = false;
    }
  }
  function counts(value: BackupCounts) {
    return t("backup.import.counts", { ...value });
  }
  function time(value: number) {
    return formatDate(value);
  }
  async function apply() {
    if (operationBusy.value || !preview.value) return;
    busy.value = true;
    error.value = "";
    try {
      if (props.initializing) {
        if (!props.initialize)
          throw new Error(t("backup.errors.setupUnavailable"));
        if (!(await props.initialize(preview.value.token))) {
          error.value = t("backup.validation.localPassword");
          return;
        }
      } else {
        const result = await desktop.applyCloudRestore(preview.value.token);
        success.value = t("backup.import.success", {
          counts: counts(result.counts),
        });
        importWarnings.value = result.warnings;
      }
      previewOpen.value = false;
      preview.value = undefined;
      config.backupPassword = "";
      config.confirmation = "";
      config.secret = "";
    } catch (cause) {
      error.value = errorMessage(cause);
    } finally {
      busy.value = false;
    }
  }
  onMounted(async () => {
    if (props.initializing) return;
    busy.value = true;
    try {
      const value = await desktop.getBackupConfig();
      if (value) acceptConfig(value);
    } catch (cause) {
      error.value = errorMessage(cause);
    } finally {
      busy.value = false;
    }
  });
  onBeforeUnmount(() => {
    config.secret = "";
    config.backupPassword = "";
    config.confirmation = "";
    preview.value = undefined;
    onBusyChange(false);
  });
  return {
    busy,
    error,
    success,
    config,
    importWarnings,
    action,
    entries,
    listed,
    hasMore,
    preview,
    previewOpen,
    operationBusy,
    canReuseSecret,
    canReuseBackupPassword,
    dirty,
    configRules,
    save,
    testConnection,
    loadMore,
    execute,
    prepare,
    counts,
    time,
    apply,
  };
}
