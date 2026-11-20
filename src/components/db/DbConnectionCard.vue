<template>
  <div class="card conn-card">
    <div class="conn-row">
      <div class="seg" role="group" aria-label="数据库类型">
        <button
          v-for="item in KINDS"
          :key="item.value"
          type="button"
          :class="{ on: kind === item.value }"
          :disabled="busy"
          :aria-pressed="kind === item.value"
          @click="emit('update:kind', item.value)"
        >
          {{ item.label }}
        </button>
      </div>

      <n-input
        :value="dsn"
        class="dsn"
        :placeholder="placeholder"
        :disabled="connecting"
        spellcheck="false"
        @update:value="emit('update:dsn', $event)"
      />

      <div
        v-if="session && session.kind !== 'sqlite'"
        class="db-picker"
        :title="session.kind === 'postgres' ? 'PostgreSQL 库由连接串指定，暂不支持在线切换' : ''"
      >
        <span class="hint">数据库</span>
        <DbDatabaseSelect
          :databases="pickerDatabases"
          :value="selectedDatabase"
          :loading="switching"
          :disabled="session.kind === 'postgres'"
          @update:value="emit('select-database', $event)"
        />
        <n-button
          class="refresh-btn"
          title="重新读取当前库结构"
          :loading="refreshing"
          :disabled="connecting || switching"
          @click="emit('refresh')"
        >
          <AppIcon v-if="!refreshing" name="refresh" :size="16" />
        </n-button>
      </div>

      <n-button
        v-if="kind === 'sqlite'"
        :loading="picking"
        :disabled="connecting || disconnecting"
        @click="emit('pick')"
      >
        <template #icon>
          <AppIcon name="folder" :size="16" />
        </template>
        选择文件
      </n-button>

      <n-button
        type="primary"
        :loading="connecting"
        :disabled="disconnecting"
        @click="emit('connect')"
      >
        <template #icon>
          <AppIcon name="zap" :size="16" />
        </template>
        连接
      </n-button>

      <n-button
        v-if="session"
        :loading="disconnecting"
        :disabled="connecting"
        @click="emit('disconnect')"
      >
        断开
      </n-button>
    </div>

    <p class="hint conn-state">{{ stateText }}</p>
    <p class="hint conn-tip">
      连接固定为只读，语句超时 15 秒；MySQL 可省略连接串末尾的库名，需要 TLS 时在连接串后加
      ?require_ssl=true。
    </p>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { NButton, NInput } from "naive-ui";
import AppIcon from "../AppIcon.vue";
import DbDatabaseSelect from "./DbDatabaseSelect.vue";
import { baseName, dbKindLabel } from "./sqlText";
import type { DbDatabase, DbKind, DbSessionInfo } from "../../types";

const props = defineProps<{
  kind: DbKind;
  dsn: string;
  session: DbSessionInfo | null;
  selectedDatabase: string;
  connecting: boolean;
  picking: boolean;
  disconnecting: boolean;
  switching: boolean;
  refreshing: boolean;
}>();

const emit = defineEmits<{
  "update:kind": [kind: DbKind];
  "update:dsn": [value: string];
  pick: [];
  connect: [];
  disconnect: [];
  refresh: [];
  "select-database": [value: string];
}>();

const KINDS: Array<{ value: DbKind; label: string }> = [
  { value: "sqlite", label: "SQLite" },
  { value: "postgres", label: "PostgreSQL" },
  { value: "mysql", label: "MySQL" },
];

const busy = computed(() => props.connecting || props.disconnecting);

const placeholder = computed(() => {
  if (props.kind === "sqlite") return "选择或输入 .db / .sqlite 文件路径";
  if (props.kind === "postgres") {
    return "postgres://user:pass@host:5432/dbname 或 host=... user=... password=...";
  }
  return "mysql://user:pass@host:3306（库名可省略）";
});

/** PostgreSQL 只有连接串指定的库，构造一条只读展示项，保证下拉里能看到当前库。 */
const pickerDatabases = computed<DbDatabase[]>(() => {
  const current = props.session;
  if (!current) return [];
  if (current.databases.length) return current.databases;
  if (current.kind === "postgres") {
    return [{ name: current.label, tableCount: current.schema.tables.length }];
  }
  return [];
});

const stateText = computed(() => {
  if (props.connecting) return `正在连接 · ${dbKindLabel(props.kind)}…`;
  const current = props.session;
  if (!current) {
    const next =
      props.kind === "sqlite" ? "选择数据库文件后点击「连接」" : "填写连接串后点击「连接」";
    return `未连接 · ${dbKindLabel(props.kind)} · ${next}。`;
  }
  const target = current.kind === "sqlite" ? baseName(current.label) : current.label;
  return `已连接 · ${dbKindLabel(current.kind)} · ${target} · 只读模式，不会写入任何数据。`;
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
.dsn {
  flex: 1;
  min-width: 240px;
}
.dsn :deep(input) {
  font-family: var(--font-mono);
  font-size: var(--fs-12);
}
.db-picker {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: none;
}
.refresh-btn {
  width: var(--h-ctl);
  padding: 0;
}
.conn-state {
  margin: 10px 0 0;
}
.conn-tip {
  margin: 2px 0 0;
  color: var(--faint);
}
.seg button:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}
</style>
