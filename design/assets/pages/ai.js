/* AI 工具视图设计稿交互：并行对比演示、用量、凭据增删改 */

const ANSWERS = {
  "OpenAI": "m3u8 是一种基于文本的 HLS 播放列表格式，通过索引一组按序编号的 TS / fMP4 分片，让播放器可以按网络状况自适应切换码率并流式播放。",
  "智谱 GLM": "m3u8 是 HLS 流媒体协议使用的播放列表文件，本质是一份 UTF-8 文本清单，记录各媒体分片的地址与时长，播放器据此逐段下载播放。",
  "DeepSeek": "m3u8 是 Apple HLS 方案的播放列表文件（UTF-8 编码的 M3U），列出媒体分片 URL 与元信息，支持多码率切换，常用于直播与点播。",
};

const KEYS = [
  { name: "OpenAI 主密钥", service: "OpenAI", secret: "sk-proj-8f2a…c41d", expire: "", note: "翻译 + LLM 调试", builtin: false },
  { name: "DeepSeek 主密钥", service: "DeepSeek", secret: "sk-9b7e…02af", expire: "2027-03-01", note: "LLM 调试台", builtin: false },
  { name: "翻译内置", service: "微软翻译", secret: "（无需密钥）", expire: "", note: "免费额度", builtin: true },
  { name: "百度翻译", service: "百度翻译", secret: "2024…b8e1", expire: "2026-12-10", note: "回退服务", builtin: false },
];

function expirePill(expire) {
  if (!expire) return '<span class="pill pill-idle">长期</span>';
  const days = Math.ceil((new Date(expire) - Date.now()) / 86400000);
  if (days < 0) return '<span class="pill pill-err">已过期</span>';
  if (days <= 30) return `<span class="pill pill-warn">${days} 天后过期</span>`;
  return `<span class="pill pill-ok">${expire}</span>`;
}

function renderKeys() {
  $("#key-count").textContent = `· ${KEYS.length}`;
  $("#key-body").innerHTML = KEYS.length ? KEYS.map((k, i) => `
    <tr data-i="${i}">
      <td><span style="font-weight:500">${k.name}</span>${k.builtin ? ' <span class="tag tag-primary">内置</span>' : ""}</td>
      <td>${k.service}</td>
      <td class="key-mask">${k.secret}</td>
      <td>${expirePill(k.expire)}</td>
      <td class="muted">${k.note || "—"}</td>
      <td style="text-align:right;white-space:nowrap">
        <button class="btn btn-ghost btn-sm btn-icon" data-act="copy" title="复制">${icon("copy", "ic-16")}</button>
        <button class="btn btn-ghost btn-sm btn-icon" data-act="edit" title="编辑">${icon("pencil", "ic-16")}</button>
        <button class="btn btn-ghost btn-sm btn-icon" data-act="del" title="删除" ${k.builtin ? "disabled" : ""}>${icon("trash", "ic-16")}</button>
      </td>
    </tr>`).join("")
    : `<tr><td colspan="6"><div class="empty">${icon("key")}<b>还没有凭据</b><p>新增一个 API Key 后即可使用对应服务。</p></div></td></tr>`;
}

function llmCard(provider, model) {
  return `<div class="llm-card rise" data-provider="${provider}">
    <div class="llm-head">
      <span class="tag tag-primary">${provider}</span>
      <span class="tag">${model}</span>
      <span class="hint meta">等待响应…</span>
      <span class="grow"></span>
      <button class="btn btn-ghost btn-sm btn-icon" data-act="copy" title="复制回答">${icon("copy", "ic-16")}</button>
    </div>
    <div class="llm-body muted">正在请求…</div>
  </div>`;
}

async function sendLlm() {
  const models = $$(".send-to:checked");
  if (!models.length) return toast("请至少选择一个模型", "info");
  const prompt = $("#prompt").value.trim();
  if (!prompt) return toast("请输入 Prompt", "info");
  const btn = $("#send-llm");
  if (btn.classList.contains("loading")) return;
  const box = $("#llm-results");
  box.innerHTML = models.map((m) => llmCard(m.dataset.provider, m.dataset.model)).join("");
  $("#llm-meta").textContent = `${models.length} 个模型并行请求中`;
  const isM3u8 = /m3u8/i.test(prompt);
  const t0 = performance.now();
  await busy(btn, 400);
  models.forEach((m, i) => {
    const provider = m.dataset.provider;
    setTimeout(() => {
      const card = $(`.llm-card[data-provider="${provider}"]`, box);
      if (!card) return;
      const ms = 420 + Math.floor(Math.random() * 900);
      const answer = isM3u8 ? ANSWERS[provider] : `（设计稿演示回答）针对「${prompt.slice(0, 40)}」的 ${provider} 模拟输出，实际应用中此处为模型真实回复。`;
      const pt = 20 + Math.floor(Math.random() * 30);
      const ct = 60 + Math.floor(Math.random() * 60);
      $(".meta", card).textContent = `${ms}ms · ${pt}+${ct}=${pt + ct} tok`;
      const body = $(".llm-body", card);
      body.textContent = answer;
      body.classList.remove("muted");
      if (i === models.length - 1) {
        $("#llm-meta").textContent = `${models.length} 个模型 · 总耗时 ${Math.round(performance.now() - t0)}ms`;
      }
    }, 500 + i * 450);
  });
}

document.addEventListener("DOMContentLoaded", () => {
  renderKeys();

  $("#send-llm").addEventListener("click", sendLlm);
  $("#clear-llm").addEventListener("click", () => {
    $("#llm-results").innerHTML = `<div class="empty">${icon("sparkles")}<b>还没有对比结果</b><p>选择模型后点击「并行请求」。</p></div>`;
    $("#llm-meta").textContent = "";
  });
  $("#llm-results").addEventListener("click", (e) => {
    const btn = e.target.closest('[data-act="copy"]');
    if (!btn) return;
    const body = $(".llm-body", btn.closest(".llm-card"));
    void copyText(body.textContent);
  });
  $("#tpl").addEventListener("change", (e) => {
    const v = e.target.value;
    if (v === "翻译") {
      $("#sys").value = "你是专业翻译，只输出译文。";
      $("#prompt").value = "把下面的内容翻译成英文：{{text}}";
    } else if (v === "代码解释") {
      $("#sys").value = "你是资深工程师，用中文解释代码。";
      $("#prompt").value = "解释这段代码的作用：{{code}}";
    }
  });

  /* 凭据管理 */
  let editingKey = null;
  $("#key-body").addEventListener("click", (e) => {
    const btn = e.target.closest("[data-act]");
    if (!btn || btn.disabled) return;
    const i = Number(btn.closest("tr").dataset.i);
    const k = KEYS[i];
    const act = btn.dataset.act;
    if (act === "copy") void copyText(k.secret);
    else if (act === "del") {
      KEYS.splice(i, 1);
      renderKeys();
      toast(`已删除「${k.name}」`);
    } else if (act === "edit") {
      editingKey = i;
      $("#key-modal h3").textContent = "编辑凭据";
      $("#nk-name").value = k.name;
      $("#nk-service").value = k.service;
      $("#nk-secret").value = "";
      $("#nk-secret").placeholder = "留空保持不变";
      $("#nk-expire").value = k.expire;
      $("#nk-note").value = k.note;
      $("#key-modal").classList.add("open");
    }
  });

  $$("[data-modal-open='key-modal']").forEach((b) => b.addEventListener("click", () => {
    editingKey = null;
    $("#key-modal h3").textContent = "新增凭据";
    ["#nk-name", "#nk-secret", "#nk-expire", "#nk-note"].forEach((s) => { $(s).value = ""; });
    $("#nk-secret").placeholder = "sk-…";
  }));

  $("#nk-create").addEventListener("click", async (e) => {
    const name = $("#nk-name").value.trim();
    const secret = $("#nk-secret").value.trim();
    if (!name) return toast("请填写名称", "info");
    if (editingKey === null && !secret) return toast("请填写密钥", "info");
    await busy(e.currentTarget, 500);
    const masked = secret ? `${secret.slice(0, 6)}…${secret.slice(-4)}` : null;
    if (editingKey !== null) {
      const k = KEYS[editingKey];
      Object.assign(k, {
        name, service: $("#nk-service").value,
        secret: masked || k.secret,
        expire: $("#nk-expire").value, note: $("#nk-note").value.trim(),
      });
      toast(`已保存「${name}」`);
    } else {
      KEYS.push({ name, service: $("#nk-service").value, secret: masked, expire: $("#nk-expire").value, note: $("#nk-note").value.trim(), builtin: false });
      toast(`已新增「${name}」`);
    }
    $("#key-modal").classList.remove("open");
    renderKeys();
  });
});