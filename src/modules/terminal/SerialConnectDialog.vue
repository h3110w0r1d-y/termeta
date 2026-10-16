<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import { RefreshOutline } from "@vicons/ionicons5";
import {
  NAlert,
  NAutoComplete,
  NButton,
  NCollapse,
  NCollapseItem,
  NFlex,
  NForm,
  NFormItem,
  NIcon,
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
  SerialConfig,
  SerialPortInfo,
} from "../../services/desktop/types";
import { validateForm } from "../../services/forms/rules";

const props = defineProps<{ open: boolean }>();
const emit = defineEmits<{
  "update:open": [boolean];
  connect: [SerialConfig];
  "after-leave": [];
}>();
const draft = reactive({
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
const flowOptions = [
  { label: "None", value: "none" },
  { label: "RTS/CTS", value: "hardware" },
  { label: "XON/XOFF", value: "software" },
];
const parityOptions = [
  { label: "No", value: "none" },
  { label: "Odd", value: "odd" },
  { label: "Even", value: "even" },
];
const rules: FormRules = {
  path: {
    required: true,
    trigger: ["input", "change", "blur"],
    validator: (_, value: string) =>
      (!!value.trim() &&
        new TextEncoder().encode(value.trim()).length <= 1024 &&
        !/[\u0000-\u001f\u007f-\u009f]/u.test(value)) ||
      new Error("请输入串口设备路径或名称，最长1024字节且不能含控制字符"),
  },
  baudRate: {
    required: true,
    trigger: ["input", "change", "blur"],
    validator: (_, value: string) =>
      (/^\d+$/.test(value) &&
        Number(value) >= 1 &&
        Number(value) <= 4_000_000) ||
      new Error("波特率须为 1–4000000 的整数"),
  },
};
async function refresh() {
  if (loading.value) return;
  const request = ++generation;
  loading.value = true;
  error.value = "";
  try {
    const items = await desktop.listSerialPorts();
    if (!props.open || request !== generation) return;
    ports.value = items;
    if (!draft.path && items.length) draft.path = items[0]!.path;
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
    if (open) {
      ports.value = [];
      form.value?.restoreValidation();
      void refresh();
    }
  },
);
async function connect() {
  if (loading.value || submitting.value) return;
  submitting.value = true;
  try {
    if (!(await validateForm(form.value)) || !props.open) return;
    emit("connect", {
      ...draft,
      path: draft.path.trim(),
      baudRate: Number(draft.baudRate),
    });
    emit("update:open", false);
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <AppDialog
    :open="open"
    title="串口连接"
    :auto-focus="false"
    @update:open="emit('update:open', $event)"
    @after-leave="emit('after-leave')"
  >
    <NForm ref="form" :model="draft" :rules="rules" @submit.prevent="connect">
      <NFormItem ref="portField" label="串口" path="path">
        <NFlex class="serial-port-row" :wrap="false" align="center">
          <NAutoComplete
            v-model:value="draft.path"
            class="serial-port-select"
            :options="portOptions"
            :render-label="renderPortLabel"
            :loading="loading"
            :disabled="submitting"
            :blur-after-select="false"
            placeholder="选择或输入串口设备路径/名称"
          />
          <NButton
            :loading="loading"
            :disabled="loading || submitting"
            aria-label="刷新串口"
            title="刷新串口"
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
        未发现串口，可手动输入设备路径或连接设备后刷新；Linux
        还需确保当前用户具有串口访问权限。
      </NAlert>
      <NFormItem label="波特率" path="baudRate">
        <NAutoComplete
          v-model:value="draft.baudRate"
          :options="baudOptions"
          :disabled="submitting"
          :blur-after-select="false"
          placeholder="选择或输入波特率（默认9600）"
        />
      </NFormItem>
      <NCollapse>
        <NCollapseItem title="高级设置" name="advanced">
          <NFormItem label="字符集 (Charset)" path="charset">
            <NSelect
              v-model:value="draft.charset"
              :options="charsets"
              filterable
            />
          </NFormItem>
          <NFormItem label="数据位 (Data bits)" path="dataBits">
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
          <NFormItem label="停止位 (Stop bits)" path="stopBits">
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
          <NFormItem label="流控 (Flow control)" path="flowControl">
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
          <NFormItem label="校验 (Parity)" path="parity">
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
    </NForm>
    <template #footer>
      <NSpace justify="end">
        <NButton @click="emit('update:open', false)">取消</NButton>
        <NButton
          type="primary"
          :loading="submitting"
          :disabled="loading || !draft.path.trim()"
          @click="connect"
        >
          连接
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
