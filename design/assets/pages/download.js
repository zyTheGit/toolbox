/* 下载视图设计稿交互：批量新建、进度模拟、取消 / 重试 / 移除 / 日志 */

let seq = 0;
const TASKS = [
  { id: ++seq, url: "https://www.bilibili.com/video/BV1xx411c7mD", name: "【教程】Tauri 桌面应用开发实战.mp4", quality: "1080p", subs: "中英字幕", status: "running", progress: 62, speed: "3.2 MB/s", done: "486 MB", total: "784 MB", eta: "01:32" },
  { id: ++seq, url: "https://example.com/live/playlist.m3u8", name: "直播回放-2026-11-20.mp4", quality: "最佳 MP4", subs: "不下载", status: "queued", progress: 0 },
  { id: ++seq, url: "https://www.youtube.com/watch?v=dQw4w9WgXcQ", name: "Lo-fi beats for coding.mp4", quality: "720p", subs: "英文", status: "completed", progress: 100, path: "D:\\Downloads\\Toolbox\\Lo-fi beats for coding.mp4", time: "今天 09:12" },
  { id: ++seq, url: "https://x.com/i/status/1234567890", name: "", quality: "1080p", subs: "不下载", status: "failed", progress: 34, time: "昨天 22:47", error: "解析失败：链接已失效或需要登录。", next: "下一步：确认链接可公开访问，或在高级选项中指定 Cookie 来源后重试。" },
];

const LOG_LINES = [
  '<span class="dim">[yt-dlp]</span> Extracting URL: https://www.bilibili.com/video/BV1xx411c7mD',
  '<span class="dim">[info]</span> BV1xx411c7mD: Downloading 1 format(s): 1080p',
  '<span class="ok">[download]</span> Destination: 【教程】Tauri 桌面应用开发实战.mp4',
  '<span class="dim">[download]</span>  62.0% of 784.00MiB at 3.20MiB/s ETA 01:32',
];

function pillOf(t) {
  const map = {
    running: '<span class="pill pill-run">下载中</span>',
    queued: '<span class="pill pill-run">排队中</span>',
    completed: '<span class="pill pill-ok">已完成</span>',
    failed: '<span class="pill pill-err">失败</span>',
    canceled: '<span class="pill pill-idle">已取消</span>',
  };
  return map[t.status] || "";
}

function cardOf(t) {
  const isActive = t.status === "running" || t.status === "queued";
  let body = "";
  if (t.status === "running") {
    body = `<div class="dl-progress progress"><i style="width:${t.progress}%"></i></div>
      <div class="dl-stats"><span>速度 <b>${t.speed}</b></span><span>剩余 <b>${t.eta}</b></span><span>已下载 <b>${t.done} / ${t.total}</b></span></div>`;
  }
  if (t.status === "failed") {
    body = `<div class="alert alert-err" style="margin-top:10px">${icon("alert", "ic-16")}<span>${t.error}<br>${t.next}</span></div>`;
  }
  if (t.status === "completed" && t.path) {
    body = `<div class="dl-meta">${icon("file", "ic-16")}<span class="mono ellipsis">${t.path}</span></div>`;
  }
  const actions = [];
  if (t.status === "running" || t.status === "queued") {
    actions.push(`<button class="btn btn-outline btn-sm" data-act="cancel">${icon("x", "ic-16")}取消</button>`);
  }
  if (t.status === "failed" || t.status === "canceled") {
    actions.push(`<button class="btn btn-outline btn-sm" data-act="retry">${icon("refresh", "ic-16")}重试</button>`);
  }
  if (t.status === "completed") {
    actions.push(`<button class="btn btn-outline btn-sm" data-act="open">${icon("folder-open", "ic-16")}打开目录</button>`);
  }
  actions.push(`<button class="btn btn-ghost btn-sm btn-icon" data-act="log" title="展开日志">${icon("terminal", "ic-16")}</button>`);
  if (!isActive) {
    actions.push(`<button class="btn btn-ghost btn-sm btn-icon" data-act="remove" title="移除记录">${icon("trash", "ic-16")}</button>`);
  }
  return `<div class="dl-card" data-id="${t.id}">
    <div class="spread">
      <div class="grow" style="min-width:0">
        <div class="row" style="gap:8px">
          ${pillOf(t)}
          <span class="dl-url ellipsis grow">${t.url}</span>
        </div>
        <div class="dl-meta">
          <span class="tag">${t.quality}</span><span class="tag">${t.subs}</span>
          ${t.name ? `<span class="ellipsis">${t.name}</span>` : ""}
          ${t.time ? `<span class="faint">${t.time}</span>` : ""}
        </div>
      </div>
      <div class="dl-actions">${actions.join("")}</div>
    </div>
    ${body}
    <div class="dl-log term" hidden>${LOG_LINES.join("\n")}</div>
  </div>`;
}

function render() {
  const active = TASKS.filter((t) => t.status === "running" || t.status === "queued");
  const history = TASKS.filter((t) => t.status !== "running" && t.status !== "queued");
  $("#active-list").innerHTML = active.length
    ? active.map(cardOf).join("")
    : `<div class="empty" style="padding:24px">${icon("inbox")}<b>暂无进行中的下载</b></div>`;
  $("#history-list").innerHTML = history.length
    ? history.map(cardOf).join("")
    : `<div class="empty" style="padding:24px">${icon("inbox")}<b>暂无下载任务</b><p>粘贴视频链接或分享文本，开始第一个下载。</p></div>`;
  $("#active-n").textContent = active.length;
  $("#history-n").textContent = history.length;
  $("#queue-count").textContent = `· ${TASKS.length}`;
}

/* 单一计时器驱动所有进度，避免重复 interval */
setInterval(() => {
  let changed = false;
  const running = TASKS.filter((t) => t.status === "running");
  running.forEach((t) => {
    t.progress = Math.min(100, t.progress + 1 + Math.random() * 2);
    const totalMb = 784;
    t.done = `${Math.round((t.progress / 100) * totalMb)} MB`;
    t.total = `${totalMb} MB`;
    const left = Math.max(0, Math.round(((100 - t.progress) / 100) * 92));
    t.eta = `${String(Math.floor(left / 60)).padStart(2, "0")}:${String(left % 60).padStart(2, "0")}`;
    if (t.progress >= 100) {
      t.status = "completed";
      t.path = `D:\\Downloads\\Toolbox\\${t.name || "video.mp4"}`;
      t.time = "刚刚";
      toast(`下载完成：${t.name || t.url}`);
      changed = true;
    }
  });
  const queued = TASKS.find((t) => t.status === "queued");
  if (queued && TASKS.filter((t) => t.status === "running").length < 2) {
    queued.status = "running";
    queued.speed = "2.8 MB/s";
    queued.done = "0 MB";
    queued.total = "784 MB";
    queued.eta = "04:30";
    changed = true;
  }
  if (changed) render();
  else {
    running.forEach((t) => {
      const card = $(`.dl-card[data-id="${t.id}"]`);
      if (!card) return;
      $(".progress i", card).style.width = `${t.progress}%`;
      const stats = $$(".dl-stats b", card);
      if (stats.length === 3) { stats[0].textContent = t.speed; stats[1].textContent = t.eta; stats[2].textContent = `${t.done} / ${t.total}`; }
    });
  }
}, 900);

function updateLinkCount() {
  const n = $("#urls").value.split("\n").filter((l) => /https?:\/\/\S+/.test(l.trim())).length;
  $("#link-count").textContent = `共 ${n} 个有效链接`;
}

document.addEventListener("DOMContentLoaded", () => {
  render();
  updateLinkCount();
  $("#urls").addEventListener("input", updateLinkCount);
  $("#open-dir").addEventListener("click", () => toast("已打开 D:\\Downloads\\Toolbox", "info"));

  $("#start-dl").addEventListener("click", async (e) => {
    const urls = $("#urls").value.split("\n").map((l) => l.trim()).filter((l) => /https?:\/\/\S+/.test(l));
    if (!urls.length) return toast("请先粘贴至少一个有效链接", "info");
    await busy(e.currentTarget, 600);
    const quality = $("#quality").value;
    const subs = $("#subs").value;
    urls.forEach((url, i) => {
      TASKS.unshift({ id: ++seq, url, name: "", quality, subs, status: i === 0 ? "running" : "queued", progress: 0, speed: "2.5 MB/s", done: "0 MB", total: "784 MB", eta: "05:00" });
    });
    render();
    toast(`已加入 ${urls.length} 个下载任务`);
  });

  document.addEventListener("click", (e) => {
    const btn = e.target.closest(".dl-card [data-act]");
    if (!btn) return;
    const card = btn.closest(".dl-card");
    const t = TASKS.find((x) => x.id === Number(card.dataset.id));
    if (!t) return;
    const act = btn.dataset.act;
    if (act === "cancel") { t.status = "canceled"; t.time = "刚刚"; render(); toast("已取消下载", "info"); }
    else if (act === "retry") { t.status = "running"; t.progress = 0; t.speed = "2.8 MB/s"; t.done = "0 MB"; t.total = "784 MB"; t.eta = "04:30"; delete t.error; render(); toast("已重新开始下载", "info"); }
    else if (act === "remove") { TASKS.splice(TASKS.indexOf(t), 1); render(); toast("已移除记录"); }
    else if (act === "open") { toast("已打开文件所在目录", "info"); }
    else if (act === "log") {
      const log = $(".dl-log", card);
      log.hidden = !log.hidden;
      btn.classList.toggle("btn-outline", !log.hidden);
    }
  });
});