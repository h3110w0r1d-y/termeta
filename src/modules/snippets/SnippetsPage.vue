<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";
import {
  NEmpty,
  NFlex,
  NForm,
  NFormItem,
  NH1,
  NIcon,
  NInput,
  NList,
  NListItem,
  NSpin,
  NText,
  NThing,
  type FormInst,
  type FormRules,
} from "naive-ui";
import { AddOutline } from "@vicons/ionicons5";
import { t } from "../../i18n";
import AppButton from "../../components/ui/AppButton.vue";
import AppDialog from "../../components/ui/AppDialog.vue";
import AppInput from "../../components/ui/AppInput.vue";
import { confirmAction } from "../../components/ui/confirmation";
import { desktop, errorMessage } from "../../services/desktop/client";
import type { Snippet } from "../../services/desktop/types";
import { requiredText, validateForm } from "../../services/forms/rules";
import { useBackupRestored } from "../backup/useBackupRestored";

const items = ref<Snippet[]>([]);
const query = ref("");
const loading = ref(true);
const busy = ref(false);
const error = ref("");
const editorOpen = ref(false);
const editing = ref<Snippet>();
const form = ref<FormInst>();
const formModel = reactive({ name: "", content: "" });
const visible = computed(() => {
  const search = query.value.trim().toLowerCase();
  return items.value.filter((item) =>
    `${item.name}\n${item.content}`.toLowerCase().includes(search),
  );
});
const rules = computed<FormRules>(() => ({
  name: requiredText("common.fields.name"),
  content: [
    requiredText("snippets.fields.content"),
    {
      trigger: ["input", "blur", "change"],
      validator: (_, value: unknown) =>
        (typeof value === "string" &&
          !value.includes("\0") &&
          new TextEncoder().encode(value).byteLength <= 64 * 1024) ||
        new Error(t("snippets.validation.contentLimit")),
    },
  ],
}));
async function load() {
  loading.value = true;
  try {
    items.value = await desktop.listSnippets();
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    loading.value = false;
  }
}
onMounted(load);
useBackupRestored(load, (cause) => {
  error.value = errorMessage(cause);
});
function edit(item?: Snippet) {
  editing.value = item;
  formModel.name = item?.name ?? "";
  formModel.content = item?.content ?? "";
  error.value = "";
  editorOpen.value = true;
}
function closeEditor(open: boolean) {
  editorOpen.value = open;
  if (!open) {
    editing.value = undefined;
    formModel.name = "";
    formModel.content = "";
  }
}
async function save() {
  if (busy.value || !(await validateForm(form.value))) return;
  busy.value = true;
  error.value = "";
  try {
    await desktop.saveSnippet({
      id: editing.value?.id,
      revision: editing.value?.revision,
      name: formModel.name.trim(),
      content: formModel.content,
    });
    closeEditor(false);
    await load();
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    busy.value = false;
  }
}
async function remove(item: Snippet) {
  if (
    busy.value ||
    !(await confirmAction(t("snippets.confirmDelete", { name: item.name })))
  )
    return;
  busy.value = true;
  error.value = "";
  try {
    await desktop.deleteSnippet(item.id, item.revision);
    await load();
  } catch (cause) {
    await load();
    error.value = errorMessage(cause);
  } finally {
    busy.value = false;
  }
}
</script>
<template>
  <section class="snippets-page home-page">
    <header class="page-heading">
      <div>
        <NH1>{{ t("common.navigation.snippets") }}</NH1>
        <NText depth="3">{{ t("snippets.description") }}</NText>
      </div>
      <AppButton class="primary" :disabled="busy" @click="edit()">
        <NIcon :size="16" aria-hidden="true"><AddOutline /></NIcon>
        {{ t("snippets.actions.new") }}
      </AppButton>
    </header>
    <AppInput
      v-model="query"
      class="snippet-search"
      type="search"
      :aria-label="t('snippets.search.label')"
      :placeholder="t('snippets.search.placeholder')"
    />
    <p v-if="error && !editorOpen" class="error" role="alert">{{ error }}</p>
    <NSpin v-if="loading" size="small" :description="t('snippets.loading')" />
    <NEmpty
      v-else-if="!visible.length"
      class="snippets-empty"
      :description="
        items.length ? t('snippets.search.noResults') : t('snippets.empty')
      "
    >
      <template #extra>
        <AppButton v-if="!items.length" :disabled="busy" @click="edit()">
          {{ t("snippets.actions.new") }}
        </AppButton>
      </template>
    </NEmpty>
    <NList v-else bordered>
      <NListItem v-for="item in visible" :key="item.id">
        <NThing :title="item.name">
          <pre class="snippet-preview">{{ item.content }}</pre>
        </NThing>
        <template #suffix>
          <div class="button-row">
            <AppButton :disabled="busy" @click="edit(item)">
              {{ t("snippets.actions.edit") }}
            </AppButton>
            <AppButton class="danger" :disabled="busy" @click="remove(item)">
              {{ t("common.actions.delete") }}
            </AppButton>
          </div>
        </template>
      </NListItem>
    </NList>
    <AppDialog
      :open="editorOpen"
      :title="editing ? t('snippets.actions.edit') : t('snippets.actions.new')"
      :description="t('snippets.form.description')"
      :locked="busy"
      @update:open="closeEditor"
    >
      <NForm
        id="snippet-editor-form"
        ref="form"
        :model="formModel"
        :rules="rules"
        label-placement="top"
        novalidate
        @submit.prevent="save"
      >
        <NFormItem :label="t('common.fields.name')" path="name">
          <AppInput
            v-model="formModel.name"
            :disabled="busy"
            maxlength="128"
            :placeholder="t('snippets.form.namePlaceholder')"
          />
        </NFormItem>
        <NFormItem :label="t('snippets.fields.content')" path="content">
          <NInput
            v-model:value="formModel.content"
            class="snippet-content-input"
            type="textarea"
            :disabled="busy"
            :autosize="{ minRows: 8, maxRows: 18 }"
            :input-props="{ spellcheck: false }"
            :placeholder="t('snippets.form.contentPlaceholder')"
          />
        </NFormItem>
        <p v-if="error" class="error" role="alert">{{ error }}</p>
      </NForm>
      <template #footer>
        <NFlex justify="end">
          <AppButton :disabled="busy" @click="closeEditor(false)">
            {{ t("common.actions.cancel") }}
          </AppButton>
          <AppButton
            type="submit"
            form="snippet-editor-form"
            class="primary"
            :disabled="busy"
          >
            {{ busy ? t("common.status.saving") : t("snippets.actions.save") }}
          </AppButton>
        </NFlex>
      </template>
    </AppDialog>
  </section>
</template>
<style scoped>
.snippet-search {
  margin-bottom: 24px;
}
.snippets-empty {
  padding: 60px 20px;
}
.snippet-preview {
  margin: 8px 0 0;
  max-height: 7.5em;
  overflow: auto;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  font: 12px/1.5 monospace;
}
.snippet-content-input :deep(textarea) {
  font-family: monospace;
}
</style>
