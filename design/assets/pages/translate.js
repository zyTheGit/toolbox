/* 翻译视图设计稿交互：方向切换、演示翻译、朗读、历史与收藏 */

const HISTORY = [
  { src: "桌面工具箱把翻译、下载和定时任务放在一个托盘应用里。", dst: "The desktop toolbox packs translation, downloads and scheduled tasks into one tray app.", dir: "中译英", provider: "微软翻译", time: "今天 10:24", fav: true },
  { src: "The download queue resumes automatically after restart.", dst: "下载队列在重启后自动恢复。", dir: "英译中", provider: "微软翻译", time: "今天 09:58", fav: false },
  { src: "请在本周五之前完成季度报告的初稿。", dst: "Please finish the first draft of the quarterly report by this Friday.", dir: "中译英", provider: "DeepL", time: "昨天 17:42", fav: true },
  { src: "Credentials are stored locally and never uploaded.", dst: "凭据保存在本机，不会上传。", dir: "英译中", provider: "微软翻译", time: "昨天 15:03", fav: false },
  { src: "这个接口支持批量请求，但需要控制并发数量。", dst: "This API supports batch requests, but concurrency must be limited.", dir: "中译英", provider: "Google", time: "11-18 20:31", fav: false },
  { src: "Fallback services are tried in order when the primary one fails.", dst: "主服务失败时，按顺序尝试回退服务。", dir: "英译中", provider: "微软翻译", time: "11-18 11:12", fav: false },
];

const DICT = new Map(HISTORY.map((h) => [h.src, h.dst]));
let currentDir = "auto";
let lastOut = "";

const srcEl = $("#src"), outEl = $("#out");

function detectDir(text) {
  if (currentDir !== "auto") return currentDir;
  return /[一-鿿]/.test(text) ? "zh2en" : "en2zh";
}

function updateCount() {
  $("#char-count").textContent = `${srcEl.value.length} 字符`;
}

function histRow(h, i, withDelete) {
  return `<div class="hist-row" data-i="${i}">
    <span class="tag">${h.dir}</span>
    <div class="hist-texts">
      <span class="src ellipsis"></span>
      <span class="dst ellipsis"></span>
    </div>
    <span class="hint" style="flex:none">${h.time}</span>
    <button class="btn btn-ghost btn-sm btn-icon star-btn ${h.fav ? "faved" : ""}" data-act="fav" title="收藏">
      ${icon("star", "ic-16")}</button>
    <button class="btn btn-ghost btn-sm btn-icon" data-act="load" title="载入">${icon("upload", "ic-16")}</button>
    <button class="btn btn-ghost btn-sm btn-icon" data-act="copy" title="复制译文">${icon("copy", "ic-16")}</button>
    ${withDelete ? `<button class="btn btn-ghost btn-sm btn-icon" data-act="del" title="删除">${icon("trash", "ic-16")}</button>` : ""}
  </div>`;
}

function fillRowTexts(root) {
  $$(".hist-row", root).forEach((row) => {
    const h = HISTORY[Number(row.dataset.i)];
    if (!h) return;
    $(".src", row).textContent = h.src;
    $(".dst", row).textContent = h.dst;
  });
}

function renderRecent() {
  const list = $("#recent-list");
  if (!HISTORY.length) {
    list.innerHTML = `<div class="empty">${icon("inbox")}<b>暂无翻译记录</b><p>完成一次翻译后会自动记录在这里。</p></div>`;
    return;
  }
  list.innerHTML = HISTORY.slice(0, 5).map((h) => histRow(h, HISTORY.indexOf(h), false)).join("");
  fillRowTexts(list);
}

function renderHistModal() {
  const q = $("#hist-search").value.trim().toLowerCase();
  const favOnly = $("#hist-fav").checked;
  const list = $("#hist-list");
  const items = HISTORY.map((h, i) => ({ h, i }))
    .filter(({ h }) => (!favOnly || h.fav) && (!q || h.src.toLowerCase().includes(q) || h.dst.toLowerCase().includes(q)));
  if (!items.length) {
    list.innerHTML = `<div class="empty">${icon("search")}<b>没有匹配的记录</b><p>换个关键词，或取消「仅收藏」筛选。</p></div>`;
    return;
  }
  list.innerHTML = items.map(({ i }) => histRow(HISTORY[i], i, true)).join("");
  fillRowTexts(list);
}

async function doTranslate() {
  const text = srcEl.value.trim();
  if (!text) return toast("请先输入要翻译的内容", "info");
  const btn = $("#do-translate");
  if (btn.classList.contains("loading")) return;
  outEl.innerHTML = `<div class="empty" style="padding:24px">${icon("refresh")}<b>正在翻译…</b></div>`;
  $(".empty .ic", outEl).style.animation = "spin 0.9s linear infinite";
  await busy(btn, 700);
  const dir = detectDir(text);
  let dst = DICT.get(text);
  let demo = false;
  if (!dst) {
    demo = true;
    dst = dir === "zh2en"
      ? "(Demo translation) " + text
      : "（设计稿演示译文）" + text;
  }
  lastOut = dst;
  const ms = 180 + Math.floor(Math.random() * 320);
  $("#out-provider").hidden = false;
  $("#out-info").textContent = `${ms}ms · ${dir === "zh2en" ? "中文 → 英文" : "英文 → 中文"}${demo ? " · 演示数据" : ""}`;
  outEl.textContent = dst;
  outEl.classList.remove("rise");
  void outEl.offsetWidth;
  outEl.classList.add("rise");
  HISTORY.unshift({ src: text, dst, dir: dir === "zh2en" ? "中译英" : "英译中", provider: "微软翻译", time: "刚刚", fav: false });
  renderRecent();
}

function speak(text, lang) {
  if (!text) return toast("没有可朗读的内容", "info");
  if (!("speechSynthesis" in window)) return toast("当前环境不支持朗读", "err");
  speechSynthesis.cancel();
  const u = new SpeechSynthesisUtterance(text);
  u.lang = lang;
  speechSynthesis.speak(u);
  toast("开始朗读", "info");
}

document.addEventListener("DOMContentLoaded", () => {
  updateCount();
  renderRecent();
  renderHistModal();
  srcEl.addEventListener("input", updateCount);
  srcEl.addEventListener("keydown", (e) => {
    if (e.ctrlKey && e.key === "Enter") void doTranslate();
  });
  $("#do-translate").addEventListener("click", doTranslate);
  $("#clear-src").addEventListener("click", () => { srcEl.value = ""; updateCount(); srcEl.focus(); });
  $("#copy-out").addEventListener("click", () => lastOut ? copyText(lastOut) : toast("暂无译文", "info"));
  $("#speak-src").addEventListener("click", () => speak(srcEl.value.trim(), /[一-鿿]/.test(srcEl.value) ? "zh-CN" : "en-US"));
  $("#speak-out").addEventListener("click", () => speak(lastOut, /[一-鿿]/.test(lastOut) ? "zh-CN" : "en-US"));
  $("#swap-btn").addEventListener("click", () => {
    if (!lastOut) return toast("暂无译文可交换", "info");
    const old = srcEl.value;
    srcEl.value = lastOut;
    lastOut = old;
    outEl.textContent = old;
    currentDir = currentDir === "zh2en" ? "en2zh" : "zh2en";
    $$("#dir-seg button").forEach((b) => b.classList.toggle("on", b.dataset.dir === currentDir));
    updateCount();
  });
  $("#dir-seg").addEventListener("click", (e) => {
    const btn = e.target.closest("button[data-dir]");
    if (!btn) return;
    currentDir = btn.dataset.dir;
    $$("#dir-seg button").forEach((b) => b.classList.toggle("on", b === btn));
  });

  document.addEventListener("click", (e) => {
    const btn = e.target.closest(".hist-row [data-act]");
    if (!btn) return;
    const row = btn.closest(".hist-row");
    const i = Number(row.dataset.i);
    const h = HISTORY[i];
    const act = btn.dataset.act;
    if (act === "fav") {
      h.fav = !h.fav;
      btn.classList.toggle("faved", h.fav);
      toast(h.fav ? "已收藏" : "已取消收藏", "info");
      renderRecent();
    } else if (act === "load") {
      srcEl.value = h.src;
      updateCount();
      $("#hist-modal").classList.remove("open");
      toast("已载入原文", "info");
    } else if (act === "copy") {
      void copyText(h.dst);
    } else if (act === "del") {
      HISTORY.splice(i, 1);
      renderHistModal();
      renderRecent();
      toast("已删除记录");
    }
  });

  $("#hist-search").addEventListener("input", renderHistModal);
  $("#hist-fav").addEventListener("change", renderHistModal);
  $("#hist-clear").addEventListener("click", () => {
    HISTORY.length = 0;
    renderHistModal();
    renderRecent();
    toast("历史已清空");
  });
});