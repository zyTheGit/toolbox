// 数据库页 SQL 文本工具：库类型展示名、标识符引用与点选表时的默认查询。

import type { DbKind, DbTable } from "../../types";

/** 数据库类型的中文展示名。 */
export function dbKindLabel(value: DbKind | ""): string {
  if (value === "sqlite") return "SQLite";
  return value === "postgres" ? "PostgreSQL" : "MySQL";
}

/** 按方言引用标识符：MySQL 用反引号，其余用双引号。 */
export function quoteIdentifier(name: string, kind: DbKind | ""): string {
  if (kind === "mysql") {
    return `\`${name.replace(/`/g, "``")}\``;
  }
  return `"${name.replace(/"/g, '""')}"`;
}

/** 点选表时填入编辑器的默认查询。 */
export function selectSql(table: DbTable, kind: DbKind | ""): string {
  const prefix = table.schema ? `${table.schema}.` : "";
  const raw =
    prefix && table.name.startsWith(prefix) ? table.name.slice(prefix.length) : table.name;
  const target =
    table.schema && raw !== table.name
      ? `${quoteIdentifier(table.schema, kind)}.${quoteIdentifier(raw, kind)}`
      : quoteIdentifier(table.name, kind);
  return `SELECT * FROM ${target} LIMIT 100`;
}
