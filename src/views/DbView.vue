<template>
  <div class="page">
    <PageHead
      title="数据库浏览器"
      subtitle="只读浏览 SQLite 文件、PostgreSQL 与 MySQL 库；写语句会被连接本身拒绝。"
    />

    <div class="card conn-card">
      <div class="conn-row">
        <n-radio-group v-model:value="kind" size="small">
          <n-radio-button value="sqlite">SQLite</n-radio-button>
          <n-radio-button value="postgres">PostgreSQL</n-radio-button>
          <n-radio-button value="mysql">MySQL</n-radio-button>
        </n-radio-group>
        <template v-if="kind === 'sqlite'">
          <n-input
            v-model:value="sqlitePath"
            size="small"
            placeholder="选择 .db / .sqlite 文件"
            class="grow"
          />
          <n-button size="small" @click="pickSqlite">选择文件</n-button>
        </template>
        <n-input
          v-else-if="kind === 'postgres'"
          v-model:value="postgresUrl"
          size="small"
          placeholder="postgres://user:pass@host:5432/dbname 或 host=... user=... password=..."
          class="grow"
        />
        <n-input
          v-else
          v-model:value="mysqlUrl"
          size="small"
          placeholder="mysql://user:pass@host:3306（库名可省略）"
          class="grow"
        />
        <n-button type="primary" size="small" :loading="connecting" @click="connect">连接</n-button>
        <n-button v-if="session" size="small" @click="disconnect">断开</n-button>
        <DbConnectionHistory ref="historyRef" @fill="fillFromHistory" @error="error = $event" />
      </div>
      <div class="conn-hint">
        连接固定为只读，PostgreSQL / MySQL 语句超时 15 秒，请使用有查询权限的账号。MySQL
        可省略连接串末尾的库名，连接后默认打开第一个库并可在右上角切换；需要 TLS 时在地址后加
        ?require_ssl=true。
      </div>
    </div>

    <n-alert v-if="error" type="error" :bordered="false" closable @close="error = ''">
      {{ error }}
    </n-alert>

    <div v-if="session" class="db-layout" :style="{ '--schema-width': `${schemaWidth}px` }">
      <DbSchemaPanel
        :tables="session.schema.tables"
        :active="activeTable"
        :empty-hint="schemaEmptyHint"
        @select="useTable"
      />
      <div
        class="db-resizer"
        :class="{ dragging }"
        title="拖动调整结构面板宽度"
        @pointerdown="startResize"
      ></div>
      <div class="db-main">
        <div class="card editor-card">
          <div class="editor-head">
            <span class="session-label" :title="session.label">
              {{ session.label }}
              <span class="badge">{{ kindLabel(session.kind) }}</span>
            </span>
            <div class="editor-actions">
              <DbDatabaseSelect
                v-if="session.databases.length > 1"
                :databases="session.databases"
                :value="selectedDatabase"
                :loading="switching"
                @update:value="switchDatabase"
              />
              <n-select v-model:value="limit" size="tiny" :options="limitOptions" class="limit-select" />
              <n-button type="primary" size="small" :loading="running" @click="run(0)">
                执行 Ctrl+Enter
              </n-button>
            </div>
          </div>
          <n-input
            v-model:value="sql"
            type="textarea"
            class="editor"
            :autosize="{ minRows: 4, maxRows: 12 }"
            placeholder="SELECT * FROM ..."
            @keydown.ctrl.enter.prevent="run(0)"
            @keydown.meta.enter.prevent="run(0)"
          />
        </div>
        <DbResultTable :result="result" :kind="session.kind" :loading="running" @page="run" />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from "vue";
import { NAlert, NButton, NInput, NRadioButton, NRadioGroup, NSelect } from "naive-ui";
import PageHead from "../components/PageHead.vue";
import DbConnectionHistory from "../components/db/DbConnectionHistory.vue";
import DbDatabaseSelect from "../components/db/DbDatabaseSelect.vue";
import DbSchemaPanel from "../components/db/DbSchemaPanel.vue";
import DbResultTable from "../components/db/DbResultTable.vue";
import { useSchemaWidth } from "../composables/useSchemaWidth";
import { api } from "../api";
import type { DbKind, DbQueryResult, DbSessionInfo, DbTable } from "../types";

const kind = ref<DbKind>("sqlite");
const sqlitePath = ref("");
const postgresUrl = ref("postgres://postgres@localhost:5432/postgres");
const mysqlUrl = ref("mysql://root@localhost:3306/mysql");
const connecting = ref(false);
const session = ref<DbSessionInfo | null>(null);
const activeTable = ref("");
const sql = ref("");
const limit = ref(100);
const running = ref(false);
const result = ref<DbQueryResult | null>(null);
const error = ref("");

const { schemaWidth, dragging, startResize } = useSchemaWidth();
const historyRef = ref<{ refresh: () => Promise<void> } | null>(null);
const selectedDatabase = ref("");
const switching = ref(false);

const limitOptions = [50, 100, 200, 500].map((value) => ({
  label: `${value} 行`,
  value,
}));

const schemaEmptyHint = computed(() => {
  const current = session.value;
  if (!current || current.schema.tables.length > 0) return "";
  return current.kind === "mysql" && current.databases.length > 1 ? "请先在上方选择数据库" : "";
});

async function pickSqlite(): Promise<void> {
  try {
    const path = await api.dbPickSqlite();
    if (path) sqlitePath.value = path;
  } catch (e) {
    error.value = String(e);
  }
}

async function connect(): Promise<void> {
  if (connecting.value) return;
  connecting.value = true;
  error.value = "";
  try {
    const connectionString =
      kind.value === "postgres"
        ? postgresUrl.value.trim()
        : kind.value === "mysql"
          ? mysqlUrl.value.trim()
          : null;
    const info = await api.dbOpen({
      kind: kind.value,
      sqlitePath: kind.value === "sqlite" ? sqlitePath.value.trim() : null,
      connectionString,
    });
    session.value = info;
    selectedDatabase.value = info.databases.some((db) => db.name === info.label) ? info.label : "";
    result.value = null;
    activeTable.value = "";
    void historyRef.value?.refresh();
    const first = info.schema.tables[0];
    if (first) useTable(first);
  } catch (e) {
    error.value = String(e);
  } finally {
    connecting.value = false;
  }
}

async function switchDatabase(database: string): Promise<void> {
  const current = session.value;
  if (!current || !database || switching.value) return;
  switching.value = true;
  error.value = "";
  try {
    const info = await api.dbSelectDatabase(current.id, database);
    session.value = info;
    selectedDatabase.value = database;
    result.value = null;
    activeTable.value = "";
    const first = info.schema.tables[0];
    if (first) useTable(first);
  } catch (e) {
    selectedDatabase.value = current.databases.some((db) => db.name === current.label) ? current.label : "";
    error.value = String(e);
  } finally {
    switching.value = false;
  }
}

function fillFromHistory(nextKind: DbKind, connection: string): void {
  kind.value = nextKind;
  if (nextKind === "sqlite") sqlitePath.value = connection;
  else if (nextKind === "postgres") postgresUrl.value = connection;
  else mysqlUrl.value = connection;
}

async function disconnect(): Promise<void> {
  const current = session.value;
  if (!current) return;
  session.value = null;
  selectedDatabase.value = "";
  result.value = null;
  activeTable.value = "";
  try {
    await api.dbClose(current.id);
  } catch {
    // 关闭失败不影响界面，进程退出时会回收连接
  }
}

function useTable(table: DbTable): void {
  activeTable.value = table.name;
  sql.value = selectSql(table);
  void run(0);
}

function kindLabel(value: DbKind): string {
  if (value === "sqlite") return "SQLite";
  return value === "postgres" ? "PostgreSQL" : "MySQL";
}

function selectSql(table: DbTable): string {
  const sessionKind = session.value?.kind ?? "";
  const prefix = table.schema ? `${table.schema}.` : "";
  const raw = prefix && table.name.startsWith(prefix) ? table.name.slice(prefix.length) : table.name;
  const target = table.schema && raw !== table.name
    ? `${quote(table.schema, sessionKind)}.${quote(raw, sessionKind)}`
    : quote(table.name, sessionKind);
  return `SELECT * FROM ${target} LIMIT 100`;
}

function quote(name: string, sessionKind: DbKind | ""): string {
  if (sessionKind === "mysql") {
    return `\`${name.replace(/`/g, "``")}\``;
  }
  return `"${name.replace(/"/g, '""')}"`;
}

async function run(offset = 0): Promise<void> {
  const current = session.value;
  if (!current || running.value) return;
  const text = sql.value.trim();
  if (!text) {
    error.value = "请输入 SQL";
    return;
  }
  running.value = true;
  error.value = "";
  try {
    result.value = await api.dbQuery(current.id, text, offset, limit.value);
  } catch (e) {
    error.value = String(e);
  } finally {
    running.value = false;
  }
}

onBeforeUnmount(() => {
  const current = session.value;
  if (current) void api.dbClose(current.id).catch(() => undefined);
});
</script>

<style scoped>
.conn-card {
  padding: 16px 20px;
}
.conn-row {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}
.grow {
  flex: 1;
  min-width: 260px;
}
.conn-hint {
  margin-top: 8px;
  font-size: 12px;
  color: var(--muted);
}
.db-layout {
  display: grid;
  grid-template-columns: var(--schema-width, 268px) 16px minmax(0, 1fr);
  row-gap: 16px;
  align-items: start;
}
.db-resizer {
  position: relative;
  align-self: stretch;
  cursor: col-resize;
  touch-action: none;
}
.db-resizer::before {
  content: "";
  position: absolute;
  top: 10px;
  bottom: 10px;
  left: 50%;
  width: 3px;
  transform: translateX(-50%);
  border-radius: 999px;
  background: #e3e7ef;
  transition: background 0.15s ease;
}
.db-resizer:hover::before,
.db-resizer.dragging::before {
  background: #b9c4da;
}
.db-layout .card {
  margin-top: 0;
}
.db-main {
  display: flex;
  flex-direction: column;
  gap: 16px;
  min-width: 0;
}
.editor-card {
  padding: 14px;
}
.editor-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 10px;
}
.session-label {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  font-weight: 600;
  color: #333b4d;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.badge {
  font-size: 11px;
  font-weight: 500;
  color: var(--primary);
  background: var(--primary-soft);
  border-radius: 4px;
  padding: 1px 6px;
  flex: none;
}
.editor-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: none;
}
.limit-select {
  width: 96px;
}
.editor :deep(textarea) {
  font-family: "Cascadia Mono", "JetBrains Mono", Consolas, monospace;
  font-size: 12px;
}
@media (max-width: 1000px) {
  .db-layout {
    grid-template-columns: minmax(0, 1fr);
  }
  .db-resizer {
    display: none;
  }
}
</style>
