<template>
  <div class="card">
    <div class="spread head">
      <div>
        <h2 class="section-title">API Key 管理器</h2>
        <p class="hint">列表默认脱敏；复制明文需二次确认。</p>
        <SecretProtectionNotice />
      </div>
      <n-button type="primary" size="small" @click="openCreate">新增凭据</n-button>
    </div>

    <div v-if="loading" class="empty"><n-spin size="small" /> 正在读取…</div>
    <div v-else-if="!store.secretEntries.length" class="empty">
      还没有凭据。翻译服务的 Key 可在设置页配置，或在右侧新增。
    </div>
    <div v-else class="key-table">
      <div class="key-row key-head">
        <span>名称</span>
        <span>服务</span>
        <span>密钥</span>
        <span>过期</span>
        <span>备注</span>
        <span class="actions-head">操作</span>
      </div>
      <div v-for="entry in store.secretEntries" :key="entry.id" class="key-row">
        <div class="name-cell">
          <span class="name" :title="entry.name">{{ entry.name }}</span>
          <n-tag v-if="entry.builtin" size="tiny" :bordered="false">翻译内置</n-tag>
        </div>
        <span class="mono service" :title="entry.service">{{ entry.service }}</span>
        <span class="mono masked">{{ entry.masked }}</span>
        <n-tag size="tiny" :bordered="false" :type="expiryType(entry)">
          {{ expiryLabel(entry) }}
        </n-tag>
        <span class="note" :title="entry.note">{{ entry.note || "—" }}</span>
        <div class="actions">
          <n-button size="tiny" quaternary @click="copyEntry(entry)">复制</n-button>
          <n-button size="tiny" quaternary @click="openEdit(entry)">编辑</n-button>
          <n-button size="tiny" quaternary type="error" @click="removeEntry(entry)">删除</n-button>
        </div>
      </div>
    </div>
    <p class="hint foot">更新于 {{ lastUpdated || "—" }}</p>
  </div>

  <n-modal
    v-model:show="editorOpen"
    preset="card"
    :title="editor.id ? '编辑凭据' : '新增凭据'"
    style="width: 520px"
  >
    <div class="editor">
      <label class="field">
        <span class="field-label">名称</span>
        <n-input v-model:value="editor.name" placeholder="例如：智谱 GLM Key" maxlength="60" />
      </label>
      <label class="field">
        <span class="field-label">所属服务</span>
        <n-input
          v-model:value="editor.service"
          placeholder="例如：glm / openai / 公司内部"
          maxlength="40"
          :disabled="editor.builtin"
        />
      </label>
      <label class="field">
        <span class="field-label">密钥</span>
        <n-input
          v-model:value="editor.secret"
          type="password"
          show-password-on="click"
          autocomplete="off"
          :placeholder="editor.id ? '留空保持不变' : '粘贴密钥内容'"
        />
      </label>
      <label class="field">
        <span class="field-label">过期日期（可选）</span>
        <n-input v-model:value="editor.expiresOn" placeholder="YYYY-MM-DD" maxlength="10" />
      </label>
      <label class="field">
        <span class="field-label">备注（可选）</span>
        <n-input v-model:value="editor.note" placeholder="用途、额度、续期方式等" maxlength="200" />
      </label>
    </div>
    <template #footer>
      <div class="modal-foot">
        <n-button :disabled="saving" @click="editorOpen = false">取消</n-button>
        <n-button type="primary" :loading="saving" @click="saveEntry">保存</n-button>
      </div>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";
import { NButton, NInput, NModal, NSpin, NTag, useDialog, useMessage } from "naive-ui";
import { api } from "../../api";
import { loadSecretEntries, loadSnapshot, store } from "../../store";
import { describeError } from "../../utils/errors";
import { copyText } from "../../utils/clipboard";
import SecretProtectionNotice from "../settings/SecretProtectionNotice.vue";
import type { SecretEntryInput, SecretEntryView } from "../../types";

const message = useMessage();
const dialog = useDialog();

const loading = ref(false);
const saving = ref(false);
const editorOpen = ref(false);
const editor = reactive({
  id: "",
  builtin: false,
  name: "",
  service: "",
  secret: "",
  note: "",
  expiresOn: "",
});

const lastUpdated = computed(() => {
  const latest = store.secretEntries
    .map((entry) => entry.updatedAt)
    .sort()
    .at(-1);
  return latest ? latest.slice(0, 10) : "";
});

async function refresh(force = true): Promise<void> {
  loading.value = true;
  try {
    await loadSecretEntries(force);
  } catch (err) {
    message.error(`读取凭据失败：${describeError(err)}`);
  } finally {
    loading.value = false;
  }
}

function openCreate(): void {
  Object.assign(editor, {
    id: "",
    builtin: false,
    name: "",
    service: "",
    secret: "",
    note: "",
    expiresOn: "",
  });
  editorOpen.value = true;
}

function openEdit(entry: SecretEntryView): void {
  Object.assign(editor, {
    id: entry.id,
    builtin: entry.builtin,
    name: entry.name,
    service: entry.service,
    secret: "",
    note: entry.note,
    expiresOn: entry.expiresOn,
  });
  editorOpen.value = true;
}

async function saveEntry(): Promise<void> {
  if (saving.value) return;
  if (!editor.name.trim()) {
    message.warning("请填写凭据名称");
    return;
  }
  if (!editor.id && !editor.secret.trim()) {
    message.warning("请粘贴密钥内容");
    return;
  }
  const input: SecretEntryInput = {
    id: editor.id,
    name: editor.name.trim(),
    service: editor.service.trim(),
    secret: editor.secret.trim(),
    note: editor.note.trim(),
    expiresOn: editor.expiresOn.trim(),
  };
  saving.value = true;
  try {
    store.secretEntries = await api.secretSave(input);
    store.secretEntriesLoaded = true;
    editorOpen.value = false;
    await loadSnapshot();
    message.success(editor.id ? "凭据已更新" : "凭据已保存");
  } catch (err) {
    message.error(`保存失败：${describeError(err)}`);
  } finally {
    saving.value = false;
  }
}

function removeEntry(entry: SecretEntryView): void {
  dialog.warning({
    title: "删除凭据",
    content: `确定删除「${entry.name}」？删除后引用它的模型或翻译服务将不可用。`,
    positiveText: "删除",
    negativeText: "取消",
    onPositiveClick: async () => {
      try {
        store.secretEntries = await api.secretDelete(entry.id);
        store.secretEntriesLoaded = true;
        await loadSnapshot();
        message.success("已删除");
      } catch (err) {
        message.error(`删除失败：${describeError(err)}`);
      }
    },
  });
}

function copyEntry(entry: SecretEntryView): void {
  dialog.warning({
    title: "复制密钥",
    content: `即将把「${entry.name}」的明文密钥复制到剪贴板，剪贴板对其它程序可见。确定继续？`,
    positiveText: "复制",
    negativeText: "取消",
    onPositiveClick: async () => {
      try {
        const secret = await api.secretReveal(entry.id);
        await copyText(secret);
        message.success("已复制到剪贴板");
      } catch (err) {
        message.error(`复制失败：${describeError(err)}`);
      }
    },
  });
}

function expiryLabel(entry: SecretEntryView): string {
  if (entry.daysLeft === null) return entry.expiresOn || "永久";
  if (entry.daysLeft < 0) return "已过期";
  if (entry.daysLeft === 0) return "今天到期";
  return `${entry.daysLeft} 天后到期`;
}

function expiryType(entry: SecretEntryView): "error" | "warning" | "default" {
  if (entry.expired) return "error";
  if (entry.daysLeft !== null && entry.daysLeft <= 7) return "warning";
  return "default";
}

onMounted(() => {
  void refresh();
});
</script>

<style scoped>
.head {
  align-items: flex-start;
}
.head .hint {
  margin: 4px 0 0;
  max-width: 640px;
}
.empty {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 32px 0;
  color: var(--muted);
  font-size: 13px;
}
.key-table {
  margin-top: 16px;
  border: 1px solid var(--border);
  border-radius: 8px;
  overflow: hidden;
}
.key-row {
  display: grid;
  grid-template-columns: 1.4fr 0.8fr 1fr 0.9fr 1.2fr 150px;
  align-items: center;
  gap: 10px;
  padding: 9px 12px;
  font-size: 13px;
}
.key-row + .key-row {
  border-top: 1px solid var(--border);
}
.key-head {
  background: var(--soft, #f6f7fb);
  color: var(--muted);
  font-size: 12px;
  font-weight: 600;
}
.name-cell {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}
.name,
.service,
.note {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.masked {
  color: var(--muted);
}
.actions {
  display: flex;
  justify-content: flex-end;
  gap: 2px;
}
.actions-head {
  text-align: right;
}
.foot {
  margin: 12px 0 0;
}
.editor {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.field-label {
  font-size: 12px;
  font-weight: 500;
  color: #4b5468;
}
.modal-foot {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
@media (max-width: 900px) {
  .key-row {
    grid-template-columns: 1fr 1fr;
  }
  .key-head {
    display: none;
  }
}
</style>
