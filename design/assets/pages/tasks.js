/* 定时任务视图设计稿交互：筛选、启停、立即运行、新建 / 编辑、运行记录 */

let seq = 0;
const TASKS = [
  {
    id: ++seq, name: "每日站会提醒", type: "提醒", cron: "每天 09:00", next: "明天 09:00", enabled: true,
    last: { status: "ok", time: "今天 09:00", dur: "0.2s" },
    runs: [
      ["定时", "ok", "今天 09:00", "0.2s", "0"],
      ["定时", "ok", "昨天 09:00", "0.2s", "0"],
      ["手动", "ok", "11-18 10:12", "0.1s", "0"],
    ],
  },
  {
    id: ++seq, name: "清理下载缓存", type: "终端", cron: "每周一 03:30", next: "周一 03:30", enabled: true,
    last: { status: "err", time: "周一 03:30", dur: "1.8s", error: "exit 1: rmdir /s /q D:\\Downloads\\Toolbox\\.cache" },
    runs: [
      ["定时", "err", "周一 03:30", "1.8s", "1"],
      ["定时", "ok", "11-10 03:30", "2.4s", "0"],
      ["手动", "ok", "11-08 15:40", "2.1s", "0"],
    ],
  },
  {
    id: ++seq, name: "每周备份数据库", type: "终端", cron: "每周日 22:00", next: "—", enabled: false,
    last: { status: "idle", time: "11-16 22:00", dur: "3.6s" },
    runs: [["定时", "ok", "11-16 22:00", "3.6s", "0"]],
  },
];

let filter = "all";
let editingId = null;

const PILL = {
  ok: '<span class="pill pill-ok">成功</span>',
  err: '<span class="pill pill-err">失败</span>',
  idle: '<span class="pill pill-idle">未运行</span>',
  run: '<span class="pill pill-run">运行中</span>',
};

function rowOf(t) {
  const typeTag = t.type === "终端" ? '<span class="tag tag-warn">终端</span>' : '<span class="tag">提醒</span>';
  const offTag = t.enabled ? "" : '<span class="tag">已停用</span>';
  const errLine = t.last.error ? `<span class="task-err">${t.last.error}</span>` : "";
  return `<div class="task-row" data-id="${t.id}">
    <div class="task-main">
      <div class="task-name">${t.name} ${typeTag} ${offTag}</div>
      <div class="task-sched">
        <span class="cron-chip">${icon("clock", "ic-16")}${t.cron}</span>
        <span>下次运行：${t.enabled ? t.next : "—"}</span>
      </div>
      <div class="task-last">
        <span>上次：</span>${PILL[t.last.status]}
        <span>${t.last.time} · 耗时 ${t.last.dur}</span>${errLine}
      </div>
    </div>
    <div class="task-side">
      <label class="switch" title="${t.enabled ? "停用" : "启用"}"><input type="checkbox" data-act="toggle" ${t.enabled ? "checked" : ""}><i></i></label>
      <button class="btn btn-outline btn-sm" data-act="run" ${t.enabled ? "" : "disabled"}>${icon("play", "ic-16")}立即运行</button>
      <button class="btn btn-ghost btn-sm btn-icon" data-act="edit" title="编辑">${icon("pencil", "ic-16")}</button>
      <button class="btn btn-ghost btn-sm btn-icon" data-act="runs" title="运行记录">${icon("history", "ic-16")}</button>
      <button class="btn btn-ghost btn-sm btn-icon" data-act="del" title="删除">${icon("trash", "ic-16")}</button>
    </div>
  </div>`;
}

function render() {
  const list = TASKS.filter((t) => filter === "all" || (filter === "on" ? t.enabled : !t.enabled));
  $("#task-list").innerHTML = list.length
    ? list.map(rowOf).join("")
    : `<div class="empty">${icon("clock")}<b>还没有定时任务</b><p>新建一个提醒或终端任务，到点自动执行。</p>
       <button class="btn btn-primary btn-sm" data-modal-open="task-modal">${icon("plus", "ic-16")}新建任务</button></div>`;
  $("#n-all").textContent = TASKS.length;
  $("#n-on").textContent = TASKS.filter((t) => t.enabled).length;
  $("#n-off").textContent = TASKS.filter((t) => !t.enabled).length;
  const allOn = TASKS.every((t) => t.enabled);
  $("#pause-all").innerHTML = allOn
    ? `${icon("pause")}全部暂停`
    : `${icon("play")}全部恢复`;
}

function openEditor(t) {
  editingId = t ? t.id : null;
  $("#task-modal h3").textContent = t ? "编辑任务" : "新建任务";
  $("#nt-name").value = t ? t.name : "";
  $("#nt-type").value = t ? t.type : "提醒";
  $("#nt-enabled").checked = t ? t.enabled : true;
  $("#task-modal").classList.add("open");
}

document.addEventListener("DOMContentLoaded", () => {
  render();

  $$(".chip[data-filter]").forEach((chip) => {
    chip.addEventListener("click", () => {
      filter = chip.dataset.filter;
      $$(".chip[data-filter]").forEach((c) => c.classList.toggle("on", c === chip));
      render();
    });
  });

  $("#pause-all").addEventListener("click", () => {
    const allOn = TASKS.every((t) => t.enabled);
    TASKS.forEach((t) => { t.enabled = !allOn; });
    render();
    toast(allOn ? "已暂停全部任务" : "已恢复全部任务", "info");
  });

  $("#task-list").addEventListener("click", async (e) => {
    const modalBtn = e.target.closest("[data-modal-open]");
    if (modalBtn) { openEditor(null); return; }
    const btn = e.target.closest("[data-act]");
    if (!btn) return;
    const row = btn.closest(".task-row");
    const t = TASKS.find((x) => x.id === Number(row.dataset.id));
    if (!t) return;
    const act = btn.dataset.act;
    if (act === "run") {
      t.last = { status: "run", time: "刚刚", dur: "—" };
      $(".task-last .pill", row).outerHTML = PILL.run;
      await busy(btn, 1200);
      t.last = { status: "ok", time: "刚刚", dur: "0.3s" };
      t.runs.unshift(["手动", "ok", "刚刚", "0.3s", "0"]);
      render();
      toast(`「${t.name}」运行成功`);
    } else if (act === "edit") {
      openEditor(t);
    } else if (act === "runs") {
      $("#runs-title").textContent = `运行记录 · ${t.name}`;
      $("#runs-body").innerHTML = t.runs.length ? t.runs.map(([trig, st, time, dur, code]) =>
        `<tr><td><span class="tag ${trig === "手动" ? "tag-primary" : ""}">${trig}</span></td>
         <td>${PILL[st]}</td><td>${time}</td><td class="num">${dur}</td><td class="num">${code}</td></tr>`).join("")
        : '<tr><td colspan="5" class="muted" style="text-align:center;padding:20px">暂无运行记录</td></tr>';
      $("#runs-modal").classList.add("open");
    } else if (act === "del") {
      TASKS.splice(TASKS.indexOf(t), 1);
      render();
      toast(`已删除「${t.name}」`);
    }
  });

  $("#task-list").addEventListener("change", (e) => {
    const input = e.target.closest('input[data-act="toggle"]');
    if (!input) return;
    const t = TASKS.find((x) => x.id === Number(input.closest(".task-row").dataset.id));
    t.enabled = input.checked;
    render();
    toast(t.enabled ? `已启用「${t.name}」` : `已停用「${t.name}」`, "info");
  });

  /* 新建 / 编辑弹窗 */
  $("#nt-mode").addEventListener("click", (e) => {
    const btn = e.target.closest("button[data-mode]");
    if (!btn) return;
    $$("#nt-mode button").forEach((b) => b.classList.toggle("on", b === btn));
    const once = btn.dataset.mode === "once";
    $("#nt-repeat-fields").hidden = once;
    $("#nt-once-field").hidden = !once;
  });
  const updatePreview = () => {
    $("#nt-preview").textContent = `含义：${$("#nt-repeat").value} ${$("#nt-time").value || "09:00"} 执行 · 接下来：明天、后天同一时间`;
  };
  $("#nt-repeat").addEventListener("change", updatePreview);
  $("#nt-time").addEventListener("input", updatePreview);

  $("#nt-create").addEventListener("click", async (e) => {
    const name = $("#nt-name").value.trim();
    if (!name) return toast("请填写任务名称", "info");
    await busy(e.currentTarget, 500);
    const once = $("#nt-once-field").hidden === false;
    const cron = once ? "单次" : `${$("#nt-repeat").value} ${$("#nt-time").value || "09:00"}`;
    const data = {
      name, type: $("#nt-type").value, cron,
      next: once ? "指定时间" : "明天同一时间",
      enabled: $("#nt-enabled").checked,
      last: { status: "idle", time: "—", dur: "—" }, runs: [],
    };
    if (editingId) {
      Object.assign(TASKS.find((x) => x.id === editingId), data);
      toast(`已保存「${name}」`);
    } else {
      TASKS.unshift({ id: ++seq, ...data });
      toast(`已创建「${name}」`);
    }
    $("#task-modal").classList.remove("open");
    render();
  });

  $$("[data-modal-open='task-modal']").forEach((b) => b.addEventListener("click", () => openEditor(null)));
});