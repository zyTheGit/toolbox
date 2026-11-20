/* 数据库视图设计稿交互：引擎/数据库切换、结构树、SQL 执行、结果分页、最近连接 */

let kind = "SQLite";
let connected = true;
let dbName = "toolbox.db";
let current = "tasks";
let page = 0;
let pageSize = 50;

const seed = () => DB_SEEDS[kind];
const db = () => seed().dbs[dbName];
const schema = () => db().schema;
const data = () => db().data;

function defaultSql(name) {
  const t = schema().find((x) => x.name === name);
  const pk = t.cols.find(([, , p]) => p);
  return `SELECT ${data()[name].cols.join(", ")} FROM ${name} ORDER BY ${pk ? pk[0] : data()[name].cols[0]};`;
}

function connState() {
  $("#conn-state").textContent = connected
    ? `已连接 · ${kind} ${seed().version}${kind === "SQLite" ? "" : ` · 数据库 ${dbName}`} · 只读模式，不会写入任何数据。`
    : `未连接 · ${kind} · 点击「连接」测试。`;
}

function fillDbOptions(selected) {
  const sel = $("#db-name");
  sel.innerHTML = seed().order.map((n) => `<option${n === selected ? " selected" : ""}>${n}</option>`).join("");
  sel.disabled = !connected;
}

function renderTree(keyword = "") {
  const q = keyword.trim().toLowerCase();
  const items = schema().filter((t) =>
    !q || t.name.toLowerCase().includes(q) || t.cols.some(([c]) => c.toLowerCase().includes(q)));
  const tables = schema().filter((t) => t.kind === "T").length;
  $("#tree-count").textContent = `${tables} 表 · ${schema().length - tables} 视图`;
  $("#tree-db").textContent = dbName;
  const tree = $("#tree");
  if (!items.length) {
    tree.innerHTML = `<div class="empty" style="padding:20px">${icon("search")}<b>没有匹配的表</b></div>`;
    return;
  }
  tree.innerHTML = items.map((t) => `
    <div class="tree-item ${q ? "open" : ""}" data-table="${t.name}">
      <div class="tree-head">
        ${icon("chevron-right")}
        <span class="tree-badge ${t.kind === "V" ? "view" : ""}">${t.kind}</span>
        <span class="grow ellipsis">${t.name}</span>
        <span class="hint">${t.cols.length}</span>
      </div>
      <div class="tree-cols">
        ${t.cols.map(([c, type, pk]) => `<div class="tree-col">${pk ? icon("key") : ""}<span>${c}</span><span class="type">${type}</span></div>`).join("")}
      </div>
    </div>`).join("");
}

function renderResult(ms) {
  const ds = data()[current];
  const totalPages = Math.max(1, Math.ceil(ds.rows.length / pageSize));
  page = Math.min(page, totalPages - 1);
  const start = page * pageSize;
  const rows = ds.rows.slice(start, start + pageSize);
  $("#result-table").innerHTML = `
    <thead><tr>${ds.cols.map((c) => `<th>${c}</th>`).join("")}</tr></thead>
    <tbody>${rows.map((r) => `<tr>${r.map((v) => {
      if (v === null) return '<td class="null">NULL</td>';
      return `<td class="${typeof v === "number" ? "num" : ""}">${v}</td>`;
    }).join("")}</tr>`).join("")}</tbody>`;
  $("#result-meta").textContent = `第 ${start + 1}–${start + rows.length} 行 · 共 ${ds.rows.length} 行 · ${ms}ms`;
  $("#page-label").textContent = `${page + 1} / ${totalPages}`;
  $("#page-prev").disabled = page === 0;
  $("#page-next").disabled = page >= totalPages - 1;
}

function loadDb(name) {
  dbName = name;
  current = schema().find((t) => data()[t.name]).name;
  page = 0;
  $("#sql").value = defaultSql(current);
  renderTree($("#tree-search").value);
  renderResult((Math.random() * 1.4 + 0.2).toFixed(2));
  connState();
}

async function runSql() {
  const sql = $("#sql").value.trim();
  if (!sql) return toast("请输入 SQL 语句", "info");
  if (!/^(select|pragma|explain|with)\b/i.test(sql)) {
    return toast("只读模式：仅允许 SELECT / PRAGMA 等查询语句", "err");
  }
  if (!connected) return toast("尚未连接，请先点击「连接」", "info");
  await busy($("#run-sql"), 500);
  const m = sql.match(/from\s+([a-z_0-9]+)/i);
  if (m) {
    const key = Object.keys(data()).find((k) => k.toLowerCase() === m[1].toLowerCase());
    if (!key) {
      return toast(`当前数据库 ${dbName} 中不存在表 ${m[1]}`, "err");
    }
    current = key;
  }
  page = 0;
  renderResult((Math.random() * 1.4 + 0.2).toFixed(2));
}

function applyKind(next) {
  kind = next;
  connected = false;
  $$("#db-kind button").forEach((b) => b.classList.toggle("on", b.textContent === kind));
  $("#db-dsn").value = kind === "SQLite"
    ? "C:\\Users\\hpee2\\AppData\\Roaming\\Toolbox\\toolbox.db"
    : kind === "PostgreSQL" ? "postgresql://readonly@127.0.0.1:5432/demo" : "mysql://readonly@127.0.0.1:3306";
  $("#db-pick").hidden = kind !== "SQLite";
  $("#db-picker").hidden = kind === "SQLite";
  $("#sql-kind").textContent = kind;
  dbName = seed().order[0];
  fillDbOptions(dbName);
  connState();
}

const RECENTS = [
  { name: "Toolbox 主库", dsn: "C:\\Users\\hpee2\\AppData\\Roaming\\Toolbox\\toolbox.db", kind: "SQLite", db: "toolbox.db" },
  { name: "本地 MySQL", dsn: "mysql://readonly@127.0.0.1:3306", kind: "MySQL", db: "blog" },
  { name: "本地测试库", dsn: "postgresql://readonly@127.0.0.1:5432/analytics", kind: "PostgreSQL", db: "analytics" },
];

document.addEventListener("DOMContentLoaded", () => {
  renderTree();
  renderResult("0.42");
  fillDbOptions(dbName);

  $("#tree-search").addEventListener("input", (e) => renderTree(e.target.value));
  $("#tree").addEventListener("click", (e) => {
    const head = e.target.closest(".tree-head");
    if (!head) return;
    const item = head.parentElement;
    item.classList.toggle("open");
    const name = item.dataset.table;
    if (data()[name]) {
      current = name;
      page = 0;
      $("#sql").value = defaultSql(name);
      renderResult((Math.random() * 1.4 + 0.2).toFixed(2));
    }
  });

  $("#run-sql").addEventListener("click", runSql);
  $("#sql").addEventListener("keydown", (e) => {
    if (e.ctrlKey && e.key === "Enter") void runSql();
  });
  $("#page-size").addEventListener("change", (e) => {
    pageSize = Number(e.target.value);
    page = 0;
    renderResult("0.31");
  });
  $("#page-prev").addEventListener("click", () => { page--; renderResult("0.18"); });
  $("#page-next").addEventListener("click", () => { page++; renderResult("0.22"); });

  $("#db-kind").addEventListener("click", (e) => {
    const btn = e.target.closest("button");
    if (btn && btn.textContent !== kind) applyKind(btn.textContent);
  });
  $("#db-pick").addEventListener("click", () => toast("设计稿演示：实际应用中打开文件选择器", "info"));
  $("#db-connect").addEventListener("click", async (e) => {
    await busy(e.currentTarget, 700);
    connected = true;
    if (kind === "SQLite") dbName = "toolbox.db";
    else if (!seed().dbs[$("#db-name").value]) dbName = seed().order[0];
    else dbName = $("#db-name").value;
    fillDbOptions(dbName);
    loadDb(dbName);
    toast("连接成功");
  });
  $("#db-name").addEventListener("change", (e) => loadDb(e.target.value));
  $("#db-refresh").addEventListener("click", async (e) => {
    await busy(e.currentTarget, 600);
    fillDbOptions(dbName);
    toast(`数据库列表已刷新 · 共 ${seed().order.length} 个`, "info");
  });

  $("#recent-list").innerHTML = RECENTS.map((r, i) => `
    <div class="spread" style="border:1px solid var(--border);border-radius:8px;padding:10px 12px">
      <div style="min-width:0">
        <div class="row" style="gap:8px"><b style="font-size:13px">${r.name}</b><span class="tag">${r.kind}</span>${r.kind === "SQLite" ? "" : `<span class="tag tag-primary">${r.db}</span>`}</div>
        <div class="mono ellipsis muted" style="margin-top:2px">${r.dsn}</div>
      </div>
      <button class="btn btn-outline btn-sm" data-recent="${i}">回填</button>
    </div>`).join("");
  $("#recent-list").addEventListener("click", (e) => {
    const btn = e.target.closest("[data-recent]");
    if (!btn) return;
    const r = RECENTS[Number(btn.dataset.recent)];
    applyKind(r.kind);
    $("#db-dsn").value = r.dsn;
    connected = true;
    fillDbOptions(r.db);
    loadDb(r.db);
    $("#recent-modal").classList.remove("open");
    toast(`已恢复连接：${r.name}`);
  });
});