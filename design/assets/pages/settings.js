/* 设置视图设计稿交互：脏状态跟踪、保存栏、命令开关联动、工具安装演示 */

let dirty = false;

function renderSaveBar() {
  const bar = $("#save-bar");
  bar.classList.toggle("dirty", dirty);
  $("#save-state").innerHTML = dirty
    ? `${icon("alert")}有未保存的修改`
    : `${icon("check")}所有修改已保存`;
  $("#save").disabled = !dirty;
  $("#discard").disabled = !dirty;
}

function markDirty() {
  if (!dirty) {
    dirty = true;
    renderSaveBar();
  }
}

document.addEventListener("DOMContentLoaded", () => {
  renderSaveBar();

  $("#settings-page").addEventListener("input", (e) => {
    if (e.target.matches("input, select, textarea")) markDirty();
  });
  $("#settings-page").addEventListener("change", (e) => {
    if (e.target.matches("input, select, textarea")) markDirty();
  });

  $("#save").addEventListener("click", async (e) => {
    if (!dirty) return;
    await busy(e.currentTarget, 800);
    dirty = false;
    renderSaveBar();
    toast("设置已保存");
  });
  $("#discard").addEventListener("click", () => {
    if (!dirty) return;
    location.reload();
  });

  /* 命令执行开关与警告联动 */
  $("#cmd-switch").addEventListener("change", (e) => {
    $("#cmd-warn").hidden = e.target.checked;
    toast(e.target.checked ? "命令执行已开启" : "命令执行已关闭", "info");
  });

  /* ffmpeg 安装演示：进度条 + 校验提示 */
  $("#install-ffmpeg").addEventListener("click", async (e) => {
    const btn = e.currentTarget;
    if (btn.disabled) return;
    btn.disabled = true;
    const bar = $("#ffmpeg-progress");
    bar.hidden = false;
    const fill = $("i", bar);
    let p = 0;
    await new Promise((resolve) => {
      const timer = setInterval(() => {
        p = Math.min(100, p + 6 + Math.random() * 10);
        fill.style.width = `${p}%`;
        if (p >= 100) { clearInterval(timer); resolve(); }
      }, 160);
    });
    $("#ffmpeg-pill").className = "pill pill-ok";
    $("#ffmpeg-pill").textContent = "就绪";
    $("#ffmpeg-ver").textContent = "7.1";
    btn.disabled = false;
    btn.className = "btn btn-outline btn-sm";
    btn.innerHTML = `${icon("refresh", "ic-16")}重新下载`;
    setTimeout(() => { bar.hidden = true; fill.style.width = "0%"; }, 800);
    toast("ffmpeg 安装完成，SHA256 校验通过");
  });

  $("#upd-ytdlp").addEventListener("click", async (e) => {
    await busy(e.currentTarget, 900);
    toast("yt-dlp 已是最新版本", "info");
  });

  $("#export-backup").addEventListener("click", async (e) => {
    await busy(e.currentTarget, 700);
    toast("备份已导出到 toolbox-backup-2026-11-20.json");
  });
  $("#import-backup").addEventListener("click", async (e) => {
    await busy(e.currentTarget, 700);
    toast("备份已恢复，重启应用后生效", "info");
  });
});