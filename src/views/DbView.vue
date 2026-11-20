<template>
  <div class="page">
    <PageHead
      title="数据库浏览器"
      icon="database"
      tag="实验"
      subtitle="只读浏览 SQLite / PostgreSQL / MySQL；MySQL 连接后可切换数据库。尚未在真实实例上验证。"
    >
      <DbConnectionHistory ref="historyRef" @fill="fillFromHistory" @error="error = $event" />
    </PageHead>

    <DbConnectionCard
      v-model:kind="kind"
      :dsn="dsn"
      :session="session"
      :selected-database="selectedDatabase"
      :connecting="connecting"
      :picking="picking"
      :disconnecting="disconnecting"
      :switching="switching"
      :refreshing="refreshing"
      @update:dsn="dsn = $event"
      @pick="pickSqlite"
      @connect="connect"
      @disconnect="disconnect"
      @refresh="refreshDatabases"
      @select-database="switchDatabase"
    />

    <n-alert
      v-if="error"
      class="page-error"
      type="error"
      :bordered="false"
      closable
      @close="error = ''"
    >
      {{ error }}
    </n-alert>

    <div
      v-if="session"
      class="db-layout"
      :class="{ closing: disconnecting }"
      :style="{ '--schema-width': `${schemaWidth}px` }"
    >
      <DbSchemaPanel
        :tables="session.schema.tables"
        :active="activeTable"
        :database="schemaDatabase"
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
        <DbQueryCard
          v-model:sql="sql"
          v-model:limit="limit"
          :kind="session.kind"
          :running="running"
          @run="run(0)"
        />
        <DbResultTable :result="result" :kind="session.kind" :loading="running" @page="run" />
      </div>
    </div>

    <div v-else class="card db-empty">
      <EmptyState
        icon="database"
        title="尚未连接数据库"
        description="选择 SQLite 文件，或填写 PostgreSQL / MySQL 连接串后点击「连接」，即可浏览结构与执行只读查询。"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from "vue";
import { NAlert } from "naive-ui";
import PageHead from "../components/PageHead.vue";
import EmptyState from "../components/EmptyState.vue";
import DbConnectionCard from "../components/db/DbConnectionCard.vue";
import DbConnectionHistory from "../components/db/DbConnectionHistory.vue";
import DbQueryCard from "../components/db/DbQueryCard.vue";
import DbSchemaPanel from "../components/db/DbSchemaPanel.vue";
import DbResultTable from "../components/db/DbResultTable.vue";
import { useSchemaWidth } from "../composables/useSchemaWidth";
import { baseName, selectSql } from "../components/db/sqlText";
import { createDbSeq } from "../utils/dbSeq";
import { api } from "../api";
import type { DbKind, DbQueryResult, DbSessionInfo, DbTable } from "../types";

const kind = ref<DbKind>("sqlite");
const sqlitePath = ref("");
const postgresUrl = ref("postgres://postgres@localhost:5432/postgres");
const mysqlUrl = ref("mysql://root@localhost:3306/mysql");
const connecting = ref(false);
const picking = ref(false);
const disconnecting = ref(false);
const refreshing = ref(false);
const switching = ref(false);
const session = ref<DbSessionInfo | null>(null);
const selectedDatabase = ref("");
const activeTable = ref("");
const sql = ref("");
const limit = ref(100);
const running = ref(false);
const result = ref<DbQueryResult | null>(null);
const error = ref("");

const { schemaWidth, dragging, startResize } = useSchemaWidth();
const historyRef = ref<InstanceType<typeof DbConnectionHistory> | null>(null);
// 请求序号：open / selectDatabase / query / close 各自递增，过期响应直接丢弃
const seq = createDbSeq();

// 三种引擎各保留自己的输入内容，切换分段时不用重新填写
const dsn = computed({
  get: () => {
    if (kind.value === "sqlite") return sqlitePath.value;
    return kind.value === "postgres" ? postgresUrl.value : mysqlUrl.value;
  },
  set: (value: string) => {
    if (kind.value === "sqlite") sqlitePath.value = value;
    else if (kind.value === "postgres") postgresUrl.value = value;
    else mysqlUrl.value = value;
  },
});

const schemaDatabase = computed(() => {
  const current = session.value;
  if (!current) return "";
  return current.kind === "sqlite" ? baseName(current.label) : current.label;
});

const schemaEmptyHint = computed(() => {
  const current = session.value;
  if (!current || current.schema.tables.length > 0) return "";
  return current.kind === "mysql" && current.databases.length > 1 ? "请先在上方选择数据库" : "";
});

/** 连接 / 切库成功后统一更新会话与当前库选中项。 */
function applySession(info: DbSessionInfo): void {
  session.value = info;
  if (info.kind === "postgres") {
    selectedDatabase.value = info.label;
    return;
  }
  selectedDatabase.value = info.databases.some((db) => db.name === info.label) ? info.label : "";
}

async function pickSqlite(): Promise<void> {
  if (picking.value) return;
  picking.value = true;
  try {
    const path = await api.dbPickSqlite();
    if (path) sqlitePath.value = path;
  } catch (e) {
    error.value = String(e);
  } finally {
    picking.value = false;
  }
}

async function connect(): Promise<void> {
  if (connecting.value || disconnecting.value) return;
  const token = seq.begin("open");
  const previousId = session.value?.id ?? "";
  // 重连 / 换引擎时作废旧会话未返回的查询与切库请求
  seq.invalidate("query");
  seq.invalidate("selectDatabase");
  running.value = false;
  switching.value = false;
  refreshing.value = false;
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
    // 期间已断连或再次连接，本次结果作废
    if (!seq.isLatest("open", token)) return;
    applySession(info);
    result.value = null;
    activeTable.value = "";
    if (previousId && previousId !== info.id) {
      // 重连成功后关掉旧会话，避免会话池堆积
      void api.dbClose(previousId).catch(() => undefined);
    }
    void historyRef.value?.refresh();
    const first = info.schema.tables[0];
    if (first) useTable(first);
  } catch (e) {
    if (seq.isLatest("open", token)) error.value = String(e);
  } finally {
    if (seq.isLatest("open", token)) connecting.value = false;
  }
}

async function switchDatabase(database: string): Promise<void> {
  const current = session.value;
  if (!current || !database || switching.value) return;
  const token = seq.begin("selectDatabase");
  // 切库后旧结果立即作废，避免旧查询迟到覆盖
  seq.invalidate("query");
  running.value = false;
  refreshing.value = false;
  switching.value = true;
  error.value = "";
  try {
    const info = await api.dbSelectDatabase(current.id, database);
    if (!seq.isLatest("selectDatabase", token) || session.value?.id !== current.id) return;
    applySession(info);
    result.value = null;
    activeTable.value = "";
    const first = info.schema.tables[0];
    if (first) useTable(first);
  } catch (e) {
    if (!seq.isLatest("selectDatabase", token)) return;
    const stillListed = current.databases.some((db) => db.name === current.label);
    selectedDatabase.value = stillListed || current.kind === "postgres" ? current.label : "";
    error.value = String(e);
  } finally {
    if (seq.isLatest("selectDatabase", token)) switching.value = false;
  }
}

/** 重新读取库列表与结构；MySQL 通过重新选择当前库触发后端刷新。 */
async function refreshDatabases(): Promise<void> {
  const current = session.value;
  if (!current || current.kind === "sqlite" || refreshing.value) return;
  const token = seq.begin("selectDatabase");
  refreshing.value = true;
  error.value = "";
  try {
    if (current.kind === "mysql") {
      const target = selectedDatabase.value || current.label;
      const info = await api.dbSelectDatabase(current.id, target);
      if (!seq.isLatest("selectDatabase", token) || session.value?.id !== current.id) return;
      applySession(info);
      if (activeTable.value && !info.schema.tables.some((t) => t.name === activeTable.value)) {
        activeTable.value = "";
        result.value = null;
      }
    } else {
      const schema = await api.dbSchema(current.id);
      if (!seq.isLatest("selectDatabase", token) || session.value?.id !== current.id) return;
      session.value = { ...current, schema };
    }
  } catch (e) {
    if (seq.isLatest("selectDatabase", token)) error.value = String(e);
  } finally {
    if (seq.isLatest("selectDatabase", token)) refreshing.value = false;
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
  if (!current || disconnecting.value) return;
  // 先作废未返回的开 / 切库 / 查询，避免迟到的响应把会话“复活”
  seq.invalidateAll();
  running.value = false;
  switching.value = false;
  refreshing.value = false;
  disconnecting.value = true;
  try {
    // 页面只维护一个会话，断开时顺手清掉可能残留的旧会话
    await api.dbCloseAll();
  } catch {
    // 关闭失败不影响界面，进程退出时会回收连接
  } finally {
    session.value = null;
    selectedDatabase.value = "";
    result.value = null;
    activeTable.value = "";
    connecting.value = false;
    disconnecting.value = false;
  }
}

function useTable(table: DbTable): void {
  activeTable.value = table.name;
  sql.value = selectSql(table, session.value?.kind ?? "");
  void run(0);
}

async function run(offset = 0): Promise<void> {
  const current = session.value;
  if (!current || running.value) return;
  const text = sql.value.trim();
  if (!text) {
    error.value = "请输入 SQL";
    return;
  }
  const token = seq.begin("query");
  running.value = true;
  error.value = "";
  try {
    const next = await api.dbQuery(current.id, text, offset, limit.value);
    // 快速切表 / 切库时只保留最后一次请求的结果
    if (!seq.isLatest("query", token) || session.value?.id !== current.id) return;
    result.value = next;
  } catch (e) {
    if (!seq.isLatest("query", token)) return;
    error.value = String(e);
  } finally {
    if (seq.isLatest("query", token)) running.value = false;
  }
}

onBeforeUnmount(() => {
  // 离开页面时关闭全部会话，确保连接不残留
  seq.invalidateAll();
  void api.dbCloseAll().catch(() => undefined);
});
</script>

<style scoped>
.page-error {
  margin-top: 16px;
}
.db-layout {
  display: grid;
  grid-template-columns: var(--schema-width, 264px) 16px minmax(0, 1fr);
  gap: 0 8px;
  margin-top: 16px;
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
  border-radius: var(--r-pill);
  background: var(--border);
}
.db-resizer:hover::before,
.db-resizer.dragging::before {
  background: var(--border-strong);
}
.db-main {
  display: flex;
  flex-direction: column;
  gap: 16px;
  min-width: 0;
}
/* 断开时后端仍在关闭会话，先锁住编辑区避免并发查询 */
.db-layout.closing {
  pointer-events: none;
  opacity: 0.6;
}
.db-empty {
  margin-top: 16px;
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
