/* ============================================================
   壳层脚本：统一图标 sprite、侧栏渲染、通用交互
   图标规范：24 网格 / 1.75 描边 / 圆头圆角，替代原先三套混用图标。
   ============================================================ */

const ICON_PATHS = {
  logo: '<rect x="3" y="8" width="18" height="12" rx="2"/><path d="M9 8V6a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2v2"/><path d="M3 13h18"/><path d="M10 13v2h4v-2"/>',
  translate: '<path d="M4 5h10"/><path d="M9 3v2"/><path d="M6 5c.6 3.6 2.8 6.4 6 8"/><path d="M11 5c-.6 3.4-2.6 6.2-5.5 8"/><path d="M4 19l4.5-10L13 19"/><path d="M6 15.5h5"/><path d="M16 8h5"/><path d="M18.5 6v2"/><path d="M15.5 8c.4 2.4 1.8 4.4 3.9 5.7"/><path d="M20 8c-.4 2.4-1.8 4.4-3.9 5.7"/>',
  clock: '<circle cx="12" cy="12" r="8"/><path d="M12 8v4l3 2"/>',
  code: '<path d="M8 6l-5 6 5 6"/><path d="M16 6l5 6-5 6"/><path d="M13.5 4l-3 16"/>',
  database: '<ellipse cx="12" cy="6" rx="7" ry="3"/><path d="M5 6v6c0 1.7 3.1 3 7 3s7-1.3 7-3V6"/><path d="M5 12v6c0 1.7 3.1 3 7 3s7-1.3 7-3v-6"/>',
  sparkles: '<path d="M12 3l1.8 5.2L19 10l-5.2 1.8L12 17l-1.8-5.2L5 10l5.2-1.8z"/><path d="M18 15l.9 2.1L21 18l-2.1.9L18 21l-.9-2.1L15 18l2.1-.9z"/>',
  download: '<path d="M12 4v10"/><path d="M8 10l4 4 4-4"/><path d="M4 18h16"/>',
  settings: '<circle cx="12" cy="12" r="3"/><path d="M19 12a7 7 0 0 0-.1-1.2l2-1.5-2-3.5-2.3 1a7 7 0 0 0-2-1.2L14.2 3h-4l-.4 2.6a7 7 0 0 0-2 1.2l-2.3-1-2 3.5 2 1.5A7 7 0 0 0 5 12c0 .4 0 .8.1 1.2l-2 1.5 2 3.5 2.3-1a7 7 0 0 0 2 1.2l.4 2.6h4l.4-2.6a7 7 0 0 0 2-1.2l2.3 1 2-3.5-2-1.5c.1-.4.1-.8.1-1.2z"/>',
  plus: '<path d="M12 5v14"/><path d="M5 12h14"/>',
  play: '<path d="M7 5l12 7-12 7z"/>',
  pause: '<path d="M8 5v14"/><path d="M16 5v14"/>',
  pencil: '<path d="M4 20l1-4L16.5 4.5a2.1 2.1 0 0 1 3 3L8 19z"/><path d="M13.5 6.5l3 3"/>',
  trash: '<path d="M4 7h16"/><path d="M9 7V5a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2"/><path d="M6 7l1 13h10l1-13"/><path d="M10 11v6"/><path d="M14 11v6"/>',
  copy: '<rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>',
  check: '<path d="M4 12.5l5 5L20 6.5"/>',
  x: '<path d="M6 6l12 12"/><path d="M18 6L6 18"/>',
  "chevron-down": '<path d="M6 9l6 6 6-6"/>',
  "chevron-right": '<path d="M9 6l6 6-6 6"/>',
  "chevron-left": '<path d="M15 6l-6 6 6 6"/>',
  search: '<circle cx="11" cy="11" r="7"/><path d="M20 20l-3.5-3.5"/>',
  folder: '<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>',
  "folder-open": '<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v1H7.5a2 2 0 0 0-1.9 1.4L3 18z"/><path d="M3 18l2.6-6.6A2 2 0 0 1 7.5 10H21l-2.6 6.6a2 2 0 0 1-1.9 1.4H5a2 2 0 0 1-2-2z"/>',
  refresh: '<path d="M20 12a8 8 0 1 1-2.3-5.6"/><path d="M20 4v4h-4"/>',
  star: '<path d="M12 3.5l2.6 5.3 5.9.9-4.2 4.1 1 5.8-5.3-2.8-5.3 2.8 1-5.8-4.2-4.1 5.9-.9z"/>',
  volume: '<path d="M4 9v6h4l5 4V5L8 9z"/><path d="M16.5 8.5a5 5 0 0 1 0 7"/><path d="M19 6a8.5 8.5 0 0 1 0 12"/>',
  swap: '<path d="M7 4v13"/><path d="M4 14l3 3 3-3"/><path d="M17 20V7"/><path d="M14 10l3-3 3 3"/>',
  terminal: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M7 9l3 3-3 3"/><path d="M13 15h4"/>',
  bell: '<path d="M6 9a6 6 0 0 1 12 0c0 5 2 6 2 6H4s2-1 2-6"/><path d="M10 19a2 2 0 0 0 4 0"/>',
  key: '<circle cx="8" cy="14" r="4"/><path d="M11 11l9-9"/><path d="M17 5l2.5 2.5"/><path d="M14 8l2 2"/>',
  shield: '<path d="M12 3l8 3v6c0 4.5-3.2 7.6-8 9-4.8-1.4-8-4.5-8-9V6z"/><path d="M9 12l2 2 4-4"/>',
  zap: '<path d="M13 2L4 14h6l-1 8 9-12h-6z"/>',
  link: '<path d="M10 14a5 5 0 0 0 7 0l3-3a5 5 0 0 0-7-7l-1.5 1.5"/><path d="M14 10a5 5 0 0 0-7 0l-3 3a5 5 0 0 0 7 7L12.5 19"/>',
  file: '<path d="M6 3h8l4 4v14H6z"/><path d="M14 3v4h4"/>',
  filter: '<path d="M4 5h16l-6 7v6l-4 2v-8z"/>',
  dots: '<circle cx="5" cy="12" r="1"/><circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/>',
  external: '<path d="M14 4h6v6"/><path d="M20 4L11 13"/><path d="M19 14v5a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h5"/>',
  upload: '<path d="M12 16V6"/><path d="M8 10l4-4 4 4"/><path d="M4 18h16"/>',
  save: '<path d="M5 3h11l5 5v13H5z"/><path d="M8 3v5h7V3"/><path d="M8 21v-7h8v7"/>',
  alert: '<path d="M12 3l10 18H2z"/><path d="M12 10v4"/><path d="M12 17.5v.5"/>',
  info: '<circle cx="12" cy="12" r="9"/><path d="M12 11v5"/><path d="M12 7.5v.5"/>',
  history: '<path d="M4 12a8 8 0 1 0 2.3-5.6"/><path d="M4 4v4h4"/><path d="M12 8v4l3 2"/>',
  list: '<path d="M8 6h13"/><path d="M8 12h13"/><path d="M8 18h13"/><path d="M4 6h.5"/><path d="M4 12h.5"/><path d="M4 18h.5"/>',
  table: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M3 10h18"/><path d="M9 10v10"/><path d="M15 10v10"/>',
  eye: '<path d="M2 12s3.5-6 10-6 10 6 10 6-3.5 6-10 6-10-6-10-6z"/><circle cx="12" cy="12" r="3"/>',
  lock: '<rect x="5" y="11" width="14" height="9" rx="2"/><path d="M8 11V8a4 4 0 0 1 8 0v3"/>',
  globe: '<circle cx="12" cy="12" r="9"/><path d="M3 12h18"/><path d="M12 3c2.5 2.6 3.8 5.7 3.8 9S14.5 18.4 12 21c-2.5-2.6-3.8-5.7-3.8-9S9.5 5.6 12 3z"/>',
  braces: '<path d="M8 4c-2 0-2 2-2 3s0 3-2 3c2 0 2 2 2 3s0 3 2 3"/><path d="M16 4c2 0 2 2 2 3s0 3 2 3c-2 0-2 2-2 3s0 3-2 3"/>',
  hash: '<path d="M9 3L7 21"/><path d="M17 3l-2 18"/><path d="M4 8h17"/><path d="M3 16h17"/>',
  wand: '<path d="M6 18L18 6"/><path d="M15 3v3"/><path d="M13.5 4.5h3"/><path d="M19 12v3"/><path d="M17.5 13.5h3"/><path d="M5 5v2"/><path d="M4 6h2"/>',
  power: '<path d="M12 3v8"/><path d="M6.3 6.5a8 8 0 1 0 11.4 0"/>',
  inbox: '<path d="M3 13l3-8h12l3 8v6H3z"/><path d="M3 13h5l2 3h4l2-3h5"/>',
};

function icon(name, cls = "") {
  return `<svg class="ic ${cls}" viewBox="0 0 24 24" aria-hidden="true"><use href="#i-${name}"></use></svg>`;
}

function injectSprite() {
  const symbols = Object.entries(ICON_PATHS)
    .map(([name, paths]) => `<symbol id="i-${name}" viewBox="0 0 24 24">${paths}</symbol>`)
    .join("");
  const holder = document.createElement("div");
  holder.innerHTML = `<svg xmlns="http://www.w3.org/2000/svg" style="display:none">${symbols}</svg>`;
  document.body.prepend(holder.firstElementChild);
}

const NAV_GROUPS = [
  { label: "常用", items: [
    { key: "translate", label: "翻译", icon: "translate", href: "translate.html" },
    { key: "download", label: "下载", icon: "download", href: "download.html", badge: "2" },
    { key: "tasks", label: "定时任务", icon: "clock", href: "tasks.html", badge: "3", soft: true },
  ]},
  { label: "工具", items: [
    { key: "dev", label: "开发工具", icon: "code", href: "dev.html" },
    { key: "db", label: "数据库", icon: "database", href: "db.html" },
    { key: "ai", label: "AI 工具", icon: "sparkles", href: "ai.html" },
  ]},
  { label: "系统", items: [
    { key: "settings", label: "设置", icon: "settings", href: "settings.html" },
  ]},
];

function renderSidebar() {
  const aside = document.getElementById("sidebar");
  if (!aside) return;
  const active = document.body.dataset.page;
  const groups = NAV_GROUPS.map((g) => {
    const items = g.items.map((it) => {
      const badge = it.badge ? `<em class="nav-badge ${it.soft ? "soft" : ""}">${it.badge}</em>` : "";
      return `<a class="nav-item ${it.key === active ? "active" : ""}" href="${it.href}"
        ${it.key === active ? 'aria-current="page"' : ""}>${icon(it.icon)}<span>${it.label}</span>${badge}</a>`;
    }).join("");
    return `<div class="nav-group">${g.label}</div>${items}`;
  }).join("");
  aside.className = "sidebar";
  aside.innerHTML = `
    <div class="brand">
      <div class="brand-mark">${icon("logo")}</div>
      <div><div class="brand-name">Toolbox<span class="brand-ver">v0.3.0</span></div><div class="brand-sub">桌面工具箱</div></div>
    </div>
    <nav class="nav">${groups}</nav>`;
}

/* ---------- 通用小工具 ---------- */
const $ = (sel, root = document) => root.querySelector(sel);
const $$ = (sel, root = document) => Array.from(root.querySelectorAll(sel));

function toast(msg, type = "ok") {
  let wrap = $(".toast-wrap");
  if (!wrap) {
    wrap = document.createElement("div");
    wrap.className = "toast-wrap";
    document.body.appendChild(wrap);
  }
  const el = document.createElement("div");
  el.className = `toast toast-${type}`;
  const ic = type === "ok" ? "check" : type === "err" ? "alert" : "info";
  el.innerHTML = `${icon(ic)}<span></span>`;
  el.lastElementChild.textContent = msg;
  wrap.appendChild(el);
  setTimeout(() => el.remove(), 2400);
}

async function copyText(text) {
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    const ta = document.createElement("textarea");
    ta.value = text;
    document.body.appendChild(ta);
    ta.select();
    document.execCommand("copy");
    ta.remove();
  }
  toast("已复制到剪贴板");
}

/* 异步按钮统一加载态：图标旋转 + 禁用，结束后恢复 */
function busy(btn, ms = 800) {
  return new Promise((resolve) => {
    if (btn.classList.contains("loading")) return resolve(false);
    btn.classList.add("loading");
    btn.disabled = true;
    let added = null;
    if (!btn.querySelector(".ic")) {
      added = document.createElement("span");
      added.innerHTML = icon("refresh");
      btn.prepend(added.firstElementChild);
    }
    setTimeout(() => {
      btn.classList.remove("loading");
      btn.disabled = false;
      if (added) added.remove();
      resolve(true);
    }, ms);
  });
}

/* 分段 tabs：data-tabs 容器 + data-tab 按钮 + data-panel 面板 */
function initTabs() {
  $$(".seg[data-tabs]").forEach((seg) => {
    seg.addEventListener("click", (e) => {
      const btn = e.target.closest("button[data-tab]");
      if (!btn) return;
      $$("button[data-tab]", seg).forEach((b) => b.classList.toggle("on", b === btn));
      const scope = seg.closest("[data-tabs-scope]") || document;
      $$("[data-panel]", scope).forEach((p) => {
        p.hidden = p.dataset.panel !== btn.dataset.tab;
      });
    });
  });
}

function initModals() {
  $$("[data-modal-open]").forEach((btn) => {
    btn.addEventListener("click", () => {
      const mask = document.getElementById(btn.dataset.modalOpen);
      if (mask) mask.classList.add("open");
    });
  });
  $$(".modal-mask").forEach((mask) => {
    mask.addEventListener("click", (e) => {
      if (e.target === mask || e.target.closest("[data-modal-close]")) mask.classList.remove("open");
    });
  });
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") $$(".modal-mask.open").forEach((m) => m.classList.remove("open"));
  });
}

function initCopyButtons() {
  $$("[data-copy]").forEach((btn) => {
    btn.addEventListener("click", () => {
      const sel = btn.dataset.copy;
      let text = sel;
      if (sel.startsWith("#")) {
        const target = $(sel);
        text = target ? (target.value ?? target.textContent) : "";
      }
      void copyText(text.trim());
    });
  });
}

document.addEventListener("DOMContentLoaded", () => {
  injectSprite();
  renderSidebar();
  initTabs();
  initModals();
  initCopyButtons();
});