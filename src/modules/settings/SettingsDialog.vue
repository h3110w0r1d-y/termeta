<script setup lang="ts">
import { ref, watch } from "vue";
import AppDialog from "../../components/ui/AppDialog.vue";
import ThemeSettings from "./ThemeSettings.vue";
import ShortcutSettings from "./ShortcutSettings.vue";
import SecuritySettings from "./SecuritySettings.vue";
import { errorMessage } from "../../services/desktop/client";
import { clone } from "../../services/data/clone";
import { validateBindings } from "../../services/commands/registry";
import { terminalRegistry } from "../terminal/registry";
import { useSettings, defaultSettings } from "./store";
import { builtinSchemes } from "./themes";
import type { AppSettings } from "./model";
const props = defineProps<{ open: boolean }>(),
  emit = defineEmits<{ "update:open": [boolean] }>();
const store = useSettings(),
  draft = ref<AppSettings>(defaultSettings()),
  tab = ref("general"),
  error = ref(""),
  busy = ref(false);
watch(
  () => props.open,
  (open) => {
    if (open) {
      draft.value = clone(store.value);
      error.value = "";
    }
  },
);
async function save() {
  busy.value = true;
  try {
    validateBindings(draft.value.keymap.bindings);
    if (
      draft.value.fontSize < 10 ||
      draft.value.fontSize > 32 ||
      draft.value.scrollback < 1000 ||
      draft.value.scrollback > 50000
    )
      throw new Error("字号须为 10–32，滚屏须为 1000–50000");
    await store.save(draft.value);
    terminalRegistry.apply();
    emit("update:open", false);
  } catch (e) {
    error.value = errorMessage(e);
  } finally {
    busy.value = false;
  }
}
</script>
<template>
  <AppDialog
    :open="open"
    title="设置"
    description="设置成功保存后统一应用。终端配色独立于应用外观。"
    :wide="true"
    @update:open="emit('update:open', $event)"
    ><nav class="settings-nav">
      <button
        v-for="item in [
          ['general', '常规'],
          ['themes', '终端配色'],
          ['shortcuts', '快捷键'],
          ['security', '信任与凭据'],
        ]"
        :key="item[0]"
        :class="{ selected: tab === item[0] }"
        @click="tab = item[0]!"
      >
        {{ item[1] }}
      </button>
    </nav>
    <div class="settings-body">
      <div v-if="tab === 'general'" class="form-grid">
        <label
          >应用外观<select v-model="draft.appearance">
            <option value="dark">深色</option>
            <option value="light">浅色</option>
            <option value="system">跟随系统</option>
          </select></label
        ><label
          >默认终端方案<select v-model="draft.defaultSchemeId">
            <option
              v-for="s in [...builtinSchemes, ...draft.customSchemes]"
              :key="s.id"
              :value="s.id"
            >
              {{ s.name }}
            </option>
          </select></label
        ><label class="span-2"
          >终端字体<input v-model="draft.fontFamily" maxlength="256" /></label
        ><label
          >字号<input
            v-model.number="draft.fontSize"
            type="number"
            min="10"
            max="32" /></label
        ><label
          >滚屏行数<input
            v-model.number="draft.scrollback"
            type="number"
            min="1000"
            max="50000" /></label
        ><label class="check span-2"
          ><input
            v-model="draft.confirmClose"
            type="checkbox"
          />关闭活动会话前确认</label
        ><label class="check span-2"
          ><input
            v-model="draft.multilinePaste"
            type="checkbox"
          />多行粘贴前确认</label
        >
        <p class="muted span-2">
          终端历史仅保留在当前窗口内存中。主机、信任和设置保存在本地；密码和私钥口令仅由系统凭据库管理。
        </p>
      </div>
      <ThemeSettings v-else-if="tab === 'themes'" v-model:settings="draft" />
      <ShortcutSettings
        v-else-if="tab === 'shortcuts'"
        v-model:settings="draft"
      />
      <SecuritySettings v-else />
    </div>
    <p v-if="error" class="error" role="alert">{{ error }}</p>
    <footer class="dialog-actions">
      <button @click="emit('update:open', false)">取消</button
      ><button class="primary" :disabled="busy" @click="save">
        {{ busy ? "保存中…" : "保存设置" }}
      </button>
    </footer></AppDialog
  >
</template>
