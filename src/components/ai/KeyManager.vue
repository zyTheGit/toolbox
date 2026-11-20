<template>
  <section class="card">
    <div class="card-head">
      <h3 class="card-title">
        <AppIcon name="key" :size="16" />凭据列表<span v-if="!loading" class="count">· {{ store.secretEntries.length }}</span>
      </h3>
      <n-button type="primary" size="small" @click="openCreate">
        <template #icon><AppIcon name="plus" :size="16" /></template>
        新增凭据
      </n-button>
    </div>

    <div v-if="loading" class="loading">
      <n-spin size="small" />
      <span>正在读取凭据…</span>
    </div>

    <EmptyState
      v-else-if="!store.secretEntries.length"
      icon="key"
      title="还没有凭据"
      description="新增一个 API Key 后即可使用对应服务。"
    />

    <div v-else class="table-wrap">
      <table class="tbl">
        <thead>
          <tr>
            <th>名称</th>
            <th>服务</th>
            <th>密钥</th>
            <th>过期</th>
            <th>备注</th>
            <th class="act-col">操作</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="entry in store.secretEntries" :key="entry.id">
            <td>
              <div class="name-cell">
                <span class="name" :title="entry.name">{{ entry.name }}</span>
                <span v-if="entry.builtin" class="tag tag-primary">内置</span>
              </div>
            </td>
            <td>{{ entry.service }}</td>
            <td class="key-mask">{{ entry.masked }}</td>
            <td>
              <span class="pill" :class="expiryPill(entry).cls">{{ expiryPill(entry).text }}</span>
            </td>
            <td class="note" :title="entry.note">{{ entry.note || "—" }}</td>
            <td>
              <div class="row-actions">
                <n-button
                  quaternary
                  circle
                  size="small"
                  title="复制密钥"
                  :loading="copyingId === entry.id"
                  @click="copyEntry(entry)"
                >
                  <template #icon><AppIcon name="copy" :size="16" /></template>
                </n-button>
                <n-button
                  quaternary
                  circle
                  size="small"
                  title="编辑"
                  @click="openEdit(entry)"
                >
                  <template #icon><AppIcon name="pencil" :size="16" /></template>
                </n-button>
                <n-button
                  quaternary
                  circle
                  size="small"
                  type="error"
                  title="删除"
                  :disabled="deletingId === entry.id"
                  @click="removeEntry(entry)"
                >
                  <template #icon><AppIcon name="trash" :size="16" /></template>
                </n-button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <p class="hint foot">
      密钥仅保存在本机数据目录，列表中始终脱敏显示；复制明文需二次确认。更新于
      {{ lastUpdated || "—" }}。
    </p>
    <SecretProtectionNotice class="foot" />
  </section>

  <n-modal
    v-model:show="editorOpen"
    preset="card"
    :title="editor.id ? '编辑凭据' : '新增凭据'"
    style="width: 520px"
  >
    <div class="stack">
      <div class="form-grid">
        <div class="field">
          <label>名称</label>
          <n-input v-model:value="editor.name" placeholder="例如：DeepSeek 备用" maxlength="60" />
        </div>
        <div class="field">
          <label>所属服务</label>
          <n-input
            v-model:value="editor.service"
            placeholder="例如：glm / openai / 公司内部"
            maxlength="40"
            :disabled="editor.builtin"
          />
        </div>
      </div>
      <div class="field">
        <label>密钥</label>
        <n-input
          v-model:value="editor.secret"
          type="password"
          show-password-on="click"
          autocomplete="off"
          :placeholder="editor.id ? '留空保持不变' : '粘贴密钥内容'"
        />
      </div>
      <div class="form-grid">
        <div class="field">
          <label>过期日期（可选）</label>
          <n-date-picker
            class="expiry-picker"
            type="date"
            value-format="yyyy-MM-dd"
            clearable
            :formatted-value="editor.expiresOn || null"
            placeholder="选择过期日期"
            @update:formatted-value="editor.expiresOn = $event ?? ''"
          />
        </div>
        <div class="field">
          <label>备注（可选）</label>
          <n-input v-model:value="editor.note" placeholder="用途、额度、续期方式等" maxlength="200" />
        </div>
      </div>
      <p class="hint">凭据只写入本机密钥文件；编辑时密钥留空表示保持原值。</p>
    </div>
    <template #footer>
      <div class="modal-foot">
        <n-button :disabled="saving" @click="editorOpen = false">取消</n-button>
        <n-button type="primary" :loading="saving" @click="saveEntry">
          <template #icon><AppIcon name="check" :size="16" /></template>
          保存凭据
        </n-button>
      </div>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";
import {
  NButton,
  NDatePicker,
  NInput,
  NModal,
  NSpin,
  useDialog,
  useMessage,
} from "naive-ui";
import AppIcon from "../AppIcon.vue";
import EmptyState from "../EmptyState.vue";
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
const copyingId = ref("");
const deletingId = ref("");
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
  if (deletingId.value) return;
  dialog.warning({
    title: "删除凭据",
    content: `确定删除「${entry.name}」？删除后引用它的模型或翻译服务将不可用。`,
    positiveText: "删除",
    negativeText: "取消",
    onPositiveClick: () => deleteEntry(entry),
  });
}

async function deleteEntry(entry: SecretEntryView): Promise<void> {
  if (deletingId.value) return;
  deletingId.value = entry.id;
  try {
    store.secretEntries = await api.secretDelete(entry.id);
    store.secretEntriesLoaded = true;
    await loadSnapshot();
    message.success(`已删除「${entry.name}」`);
  } catch (err) {
    message.error(`删除失败：${describeError(err)}`);
  } finally {
    deletingId.value = "";
  }
}

function copyEntry(entry: SecretEntryView): void {
  if (copyingId.value) return;
  dialog.warning({
    title: "复制密钥",
    content: `即将把「${entry.name}」的明文密钥复制到剪贴板，剪贴板对其它程序可见。确定继续？`,
    positiveText: "复制",
    negativeText: "取消",
    onPositiveClick: () => revealAndCopy(entry),
  });
}

async function revealAndCopy(entry: SecretEntryView): Promise<void> {
  if (copyingId.value) return;
  copyingId.value = entry.id;
  try {
    const secret = await api.secretReveal(entry.id);
    await copyText(secret);
    message.success("已复制到剪贴板");
  } catch (err) {
    message.error(`复制失败：${describeError(err)}`);
  } finally {
    copyingId.value = "";
  }
}

/** 过期状态按设计稿口径：无日期=长期，30 天内=预警，其余展示日期。 */
function expiryPill(entry: SecretEntryView): { text: string; cls: string } {
  if (!entry.expiresOn) return { text: "长期", cls: "pill-idle" };
  if (entry.expired) return { text: "已过期", cls: "pill-err" };
  const days = entry.daysLeft;
  if (days === null) return { text: entry.expiresOn, cls: "pill-ok" };
  if (days === 0) return { text: "今天到期", cls: "pill-warn" };
  if (days <= 30) return { text: `${days} 天后过期`, cls: "pill-warn" };
  return { text: entry.expiresOn, cls: "pill-ok" };
}

onMounted(() => {
  void refresh();
});
</script>

<style scoped>
.loading {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 36px 0;
  color: var(--muted);
  font-size: var(--fs-13);
}
.name-cell {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}
.name {
  max-width: 220px;
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.key-mask {
  font-family: var(--font-mono);
  font-size: var(--fs-12);
  color: var(--muted);
  white-space: nowrap;
}
.note {
  max-width: 220px;
  color: var(--muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.act-col {
  width: 132px;
  text-align: right;
}
.row-actions {
  display: flex;
  justify-content: flex-end;
  gap: 2px;
}
.foot {
  margin: 12px 0 0;
}
.expiry-picker {
  width: 100%;
}
.modal-foot {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
</style>
