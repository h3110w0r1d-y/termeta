<script setup lang="ts">
import { t } from "../../i18n";
import { computed, reactive, ref, watch } from "vue";
import { RefreshOutline } from "@vicons/ionicons5";
import {
  NAlert,
  NAutoComplete,
  NButton,
  NCheckbox,
  NCollapse,
  NCollapseItem,
  NFlex,
  NForm,
  NFormItem,
  NIcon,
  NInput,
  NRadioButton,
  NRadioGroup,
  NSelect,
  NSpace,
  type FormInst,
  type FormItemInst,
  type FormRules,
  type SelectOption,
} from "naive-ui";
import AppDialog from "../../components/ui/AppDialog.vue";
import { desktop, errorMessage } from "../../services/desktop/client";
import type {
  HostDraft,
  HostProfile,
  SerialConfig,
  SerialPortInfo,
} from "../../services/desktop/types";
import { requiredText, validateForm } from "../../services/forms/rules";
import { clone } from "../../services/data/clone";
import { useHosts } from "../hosts/store";
import { uniqueHostNameRule } from "../hosts/validation";
import { groupChoices } from "../hosts/groups";
import { useSettings } from "../settings/store";
import { builtinSchemes } from "../settings/themes";

const props = defineProps<{
  open: boolean;
  save?: boolean;
  host?: HostProfile;
  duplicate?: boolean;
  initialGroupId?: string | null;
}>();
const emit = defineEmits<{
  "update:open": [boolean];
  connect: [SerialConfig];
  saved: [];
  "after-leave": [];
}>();
const hosts = useHosts();
const settings = useSettings();
const draft = reactive({
  name: "",
  groupId: "",
  tagText: "",
  favorite: false,
  hostScheme: "",
  path: "",
  baudRate: "9600",
  charset: "UTF-8",
  dataBits: 8 as SerialConfig["dataBits"],
  stopBits: 1 as SerialConfig["stopBits"],
  flowControl: "none" as SerialConfig["flowControl"],
  parity: "none" as SerialConfig["parity"],
});
const form = ref<FormInst>();
const portField = ref<FormItemInst>();
const ports = ref<SerialPortInfo[]>([]);
const loading = ref(false);
const submitting = ref(false);
const error = ref("");
const saveError = ref("");
const committedHost = ref<HostProfile | null>(null);
const fieldsLocked = computed(() => submitting.value || !!committedHost.value);
const groupOptions = computed(() => [
  { value: "", label: t("未分组") },
  ...groupChoices(hosts.groups),
]);
const schemeOptions = computed(() => [
  { value: "", label: t("跟随全局默认") },
  ...[...builtinSchemes, ...settings.value.customSchemes].map((scheme) => ({
    value: scheme.id,
    label: scheme.name,
  })),
]);
let generation = 0;
const portOptions = computed(() =>
  ports.value.map((port) => ({
    value: port.path,
    label: port.path,
  })),
);
// AutoComplete会把label写入输入框，设备描述仅用于渲染候选项。
function renderPortLabel(option: SelectOption) {
  const port = ports.value.find((port) => port.path === option.value);
  return port?.description
    ? `${port.path} · ${port.description}`
    : String(option.value);
}
const baudOptions = [
  50, 75, 110, 134, 150, 200, 300, 600, 1200, 1800, 2400, 4800, 9600, 19200,
  38400, 57600, 115200, 230400, 460800, 921600,
].map(String);
const charsets = [
  "UTF-8",
  "GBK",
  "GB18030",
  "Big5",
  "Shift_JIS",
  "EUC-JP",
  "EUC-KR",
  "windows-1251",
  "windows-1252",
  "KOI8-R",
].map((value) => ({ label: value, value }));
const bits = [8, 7, 6, 5].map((value) => ({ label: String(value), value }));
const stopBits = [1, 2].map((value) => ({ label: String(value), value }));
const flowOptions = computed(() => [
  { label: t("无"), value: "none" },
  { label: "RTS/CTS", value: "hardware" },
  { label: "XON/XOFF", value: "software" },
]);
const parityOptions = computed(() => [
  { label: t("无"), value: "none" },
  { label: t("奇校验"), value: "odd" },
  { label: t("偶校验"), value: "even" },
]);
const rules = computed<FormRules>(() => ({
  name: props.save
    ? [
        requiredText(t("名称")),
        uniqueHostNameRule(
          () => hosts.hosts,
          () => (props.duplicate ? null : props.host?.id),
        ),
      ]
    : [],
  path: {
    required: true,
    trigger: ["input", "change", "blur"],
    validator: (_, value: string) =>
      (!!value.trim() &&
        new TextEncoder().encode(value.trim()).length <= 1024 &&
        !/[\u0000-\u001f\u007f-\u009f]/u.test(value)) ||
      new Error(t("请输入串口设备路径或名称，最长1024字节且不能含控制字符")),
  },
  baudRate: {
    required: true,
    trigger: ["input", "change", "blur"],
    validator: (_, value: string) =>
      (/^\d+$/.test(value) &&
        Number(value) >= 1 &&
        Number(value) <= 4_000_000) ||
      new Error(t("波特率须为 1–4000000 的整数")),
  },
}));
async function refresh() {
  if (loading.value) return;
  const request = ++generation;
  loading.value = true;
  error.value = "";
  try {
    const items = await desktop.listSerialPorts();
    if (!props.open || request !== generation) return;
    ports.value = items;
    if (!draft.path && items.length && !fieldsLocked.value)
      draft.path = items[0]!.path;
    // 枚举/默认选择是程序更新，使用库公开API重验当前共享字段。
    if (draft.path) void portField.value?.validate().catch(() => {});
  } catch (cause) {
    if (props.open && request === generation) {
      ports.value = [];
      error.value = errorMessage(cause);
    }
  } finally {
    if (request === generation) loading.value = false;
  }
}
watch(
  () => props.open,
  (open) => {
    generation++;
    loading.value = false;
    submitting.value = false;
    error.value = "";
    saveError.value = "";
    if (open) {
      const host = props.save ? props.host : undefined;
      const config = host?.serialConfig;
      // 每次打开都重建当前草稿，编辑和复制保留保存时的全部串口参数。
      Object.assign(draft, {
        name: host
          ? props.duplicate
            ? t("{name} 副本", { name: host.name })
            : host.name
          : "",
        groupId: host ? (host.groupId ?? "") : (props.initialGroupId ?? ""),
        tagText: host?.tags.join(", ") ?? "",
        favorite: host?.favorite ?? false,
        hostScheme: host ? (settings.value.hostSchemes[host.id] ?? "") : "",
        path: config?.path ?? "",
        baudRate: String(config?.baudRate ?? 9600),
        charset: config?.charset ?? "UTF-8",
        dataBits: config?.dataBits ?? 8,
        stopBits: config?.stopBits ?? 1,
        flowControl: config?.flowControl ?? "none",
        parity: config?.parity ?? "none",
      });
      committedHost.value = null;
      ports.value = [];
      form.value?.restoreValidation();
      void refresh();
    } else {
      form.value?.restoreValidation();
    }
  },
);
async function connect() {
  if (submitting.value) return;
  submitting.value = true;
  saveError.value = "";
  try {
    if (!committedHost.value && !(await validateForm(form.value))) return;
    if (!props.open) return;
    const config: SerialConfig = {
      path: draft.path.trim(),
      baudRate: Number(draft.baudRate),
      charset: draft.charset,
      dataBits: draft.dataBits,
      stopBits: draft.stopBits,
      flowControl: draft.flowControl,
      parity: draft.parity,
    };
    if (props.save) {
      const host: HostDraft = {
        id: props.host && !props.duplicate ? props.host.id : null,
        kind: "serial",
        icon: props.host?.icon ?? "ssh",
        name: draft.name.trim(),
        groupId: draft.groupId || null,
        favorite: draft.favorite,
        tags: draft.tagText
          .split(/[,，]/)
          .map((tag) => tag.trim())
          .filter(Boolean),
        address: "",
        port: 0,
        username: "",
        authPlan: [],
        serialConfig: config,
      };
      // 设备枚举只辅助选择；保存仅提交配置，不打开串口设备。
      const saved =
        committedHost.value ??
        (await hosts.save(
          host,
          props.host && !props.duplicate ? props.host.revision : null,
        ));
      committedHost.value = saved;
      const next = clone(settings.value);
      if (draft.hostScheme) next.hostSchemes[saved.id] = draft.hostScheme;
      else delete next.hostSchemes[saved.id];
      await settings.save(next);
      emit("saved");
    } else {
      emit("connect", config);
    }
    emit("update:open", false);
  } catch (cause) {
    saveError.value = committedHost.value
      ? t("主机已保存，但配色保存失败。请重试。{error}", {
          error: errorMessage(cause),
        })
      : errorMessage(cause);
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <AppDialog
    :open="open"
    :title="
      save
        ? host && !duplicate
          ? t('编辑串口主机')
          : t('新建串口主机')
        : t('串口连接')
    "
    :locked="submitting"
    :auto-focus="false"
    @update:open="emit('update:open', $event)"
    @after-leave="emit('after-leave')"
  >
    <NForm
      ref="form"
      :model="draft"
      :rules="rules"
      :disabled="fieldsLocked"
      @submit.prevent="connect"
    >
      <NFormItem v-if="save" :label="t('名称')" path="name">
        <NInput
          v-model:value="draft.name"
          maxlength="120"
          :placeholder="t('例如：开发板串口')"
        />
      </NFormItem>
      <NFormItem ref="portField" :label="t('串口')" path="path">
        <NFlex class="serial-port-row" :wrap="false" align="center">
          <NAutoComplete
            v-model:value="draft.path"
            class="serial-port-select"
            :options="portOptions"
            :render-label="renderPortLabel"
            :loading="loading"
            :disabled="fieldsLocked"
            :blur-after-select="false"
            :placeholder="t('选择或输入串口设备路径/名称')"
          />
          <NButton
            :loading="loading"
            :disabled="loading || fieldsLocked"
            :aria-label="t('刷新串口')"
            :title="t('刷新串口')"
            @click="refresh"
          >
            <template #icon>
              <NIcon :component="RefreshOutline" aria-hidden="true" />
            </template>
          </NButton>
        </NFlex>
      </NFormItem>
      <NAlert v-if="error" type="error">{{ error }}</NAlert>
      <NAlert v-else-if="!loading && !ports.length" type="info">
        {{
          t(
            "未发现串口。请连接设备后刷新，或手动输入设备名称。Linux 用户还需检查串口访问权限。",
          )
        }}
      </NAlert>
      <NFormItem :label="t('波特率')" path="baudRate">
        <NAutoComplete
          v-model:value="draft.baudRate"
          :options="baudOptions"
          :disabled="fieldsLocked"
          :blur-after-select="false"
          :placeholder="t('选择或输入波特率（默认9600）')"
        />
      </NFormItem>
      <NCollapse>
        <NCollapseItem :title="t('高级设置')" name="advanced">
          <NFormItem :label="t('字符集')" path="charset">
            <NSelect
              v-model:value="draft.charset"
              :options="charsets"
              filterable
            />
          </NFormItem>
          <NFormItem :label="t('数据位')" path="dataBits">
            <NRadioGroup v-model:value="draft.dataBits" name="serial-data-bits">
              <NRadioButton
                v-for="option in bits"
                :key="option.value"
                :value="option.value"
              >
                {{ option.label }}
              </NRadioButton>
            </NRadioGroup>
          </NFormItem>
          <NFormItem :label="t('停止位')" path="stopBits">
            <NRadioGroup v-model:value="draft.stopBits" name="serial-stop-bits">
              <NRadioButton
                v-for="option in stopBits"
                :key="option.value"
                :value="option.value"
              >
                {{ option.label }}
              </NRadioButton>
            </NRadioGroup>
          </NFormItem>
          <NFormItem :label="t('流控')" path="flowControl">
            <NRadioGroup
              v-model:value="draft.flowControl"
              name="serial-flow-control"
            >
              <NRadioButton
                v-for="option in flowOptions"
                :key="option.value"
                :value="option.value"
              >
                {{ option.label }}
              </NRadioButton>
            </NRadioGroup>
          </NFormItem>
          <NFormItem :label="t('校验')" path="parity">
            <NRadioGroup v-model:value="draft.parity" name="serial-parity">
              <NRadioButton
                v-for="option in parityOptions"
                :key="option.value"
                :value="option.value"
              >
                {{ option.label }}
              </NRadioButton>
            </NRadioGroup>
          </NFormItem>
        </NCollapseItem>
      </NCollapse>
      <template v-if="save">
        <NFormItem :label="t('主机终端配色')" path="hostScheme">
          <NSelect v-model:value="draft.hostScheme" :options="schemeOptions" />
        </NFormItem>
        <NFormItem :label="t('分组')" path="groupId">
          <NSelect v-model:value="draft.groupId" :options="groupOptions" />
        </NFormItem>
        <NFormItem :label="t('标签')" path="tagText">
          <NInput
            v-model:value="draft.tagText"
            :placeholder="t('开发板, 调试')"
          />
        </NFormItem>
        <NFormItem>
          <NCheckbox v-model:checked="draft.favorite">
            {{ t("加入收藏") }}
          </NCheckbox>
        </NFormItem>
      </template>
      <NAlert v-if="saveError" type="error">{{ saveError }}</NAlert>
    </NForm>
    <template #footer>
      <NSpace justify="end">
        <NButton :disabled="submitting" @click="emit('update:open', false)">
          {{ committedHost ? t("关闭") : t("取消") }}
        </NButton>
        <NButton
          type="primary"
          :loading="submitting"
          :disabled="submitting || !draft.path.trim()"
          @click="connect"
        >
          {{
            committedHost ? t("重试保存配色") : save ? t("保存主机") : t("连接")
          }}
        </NButton>
      </NSpace>
    </template>
  </AppDialog>
</template>

<style scoped>
.serial-port-row {
  width: 100%;
}
.serial-port-select {
  flex: 1;
  min-width: 0;
}
</style>
