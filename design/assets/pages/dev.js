/* 开发工具视图设计稿交互：JSON / 编解码 / 正则 / curl / 辅助工具，全部本地真实计算 */

/* ---------- JSON ---------- */
function jsonParse() {
  const text = $("#json-in").value.trim();
  if (!text) throw new Error("请输入 JSON 内容");
  return JSON.parse(text);
}

function showJsonErr(err) {
  const box = $("#json-err");
  box.hidden = false;
  box.innerHTML = `${icon("alert", "ic-16")}<span></span>`;
  $("span", box).textContent = `JSON 解析失败：${err.message}`;
}

function jsonIndent() {
  const v = $("#json-indent").value;
  return v === "tab" ? "\t" : Number(v);
}

function tsTypeOf(value, name, out) {
  if (value === null) return "null";
  if (Array.isArray(value)) {
    const inner = value.length ? tsTypeOf(value[0], name + "Item", out) : "unknown";
    return `${inner}[]`;
  }
  if (typeof value === "object") {
    const iface = name.charAt(0).toUpperCase() + name.slice(1);
    const lines = Object.entries(value).map(([k, v]) => `  ${k}: ${tsTypeOf(v, k, out)};`);
    out.push(`interface ${iface} {\n${lines.join("\n")}\n}`);
    return iface;
  }
  return typeof value;
}

function initJson() {
  const run = (fn) => {
    $("#json-err").hidden = true;
    try {
      $("#json-out").textContent = fn(jsonParse());
    } catch (err) {
      showJsonErr(err);
    }
  };
  $("#json-format").addEventListener("click", () => run((v) => JSON.stringify(v, null, jsonIndent())));
  $("#json-minify").addEventListener("click", () => run((v) => JSON.stringify(v)));
  $("#json-ts").addEventListener("click", () => run((v) => {
    const out = [];
    tsTypeOf(v, "root", out);
    return out.reverse().join("\n\n");
  }));
  $("#json-clear").addEventListener("click", () => {
    $("#json-in").value = "";
    $("#json-out").textContent = "// 已清空";
    $("#json-err").hidden = true;
  });
}

/* ---------- 编解码 ---------- */
function initCodec() {
  const tick = () => { $("#ts-now").textContent = Math.floor(Date.now() / 1000); };
  tick();
  setInterval(tick, 1000);
  $("#ts-convert").addEventListener("click", () => {
    const raw = $("#ts-in").value.trim();
    if (!/^\d+$/.test(raw)) { $("#ts-out").textContent = "请输入纯数字时间戳。"; return; }
    const ms = raw.length > 10 ? Number(raw) : Number(raw) * 1000;
    const d = new Date(ms);
    $("#ts-out").textContent = isNaN(d.getTime())
      ? "时间戳超出范围。"
      : `本地时间：${d.toLocaleString("zh-CN", { hour12: false })} · ISO：${d.toISOString()}`;
  });

  $("#uuid-gen").addEventListener("click", () => {
    const list = $("#uuid-list");
    list.innerHTML = "";
    for (let i = 0; i < 5; i++) {
      const u = crypto.randomUUID();
      const row = document.createElement("div");
      row.className = "row";
      row.innerHTML = `<span class="mono grow ellipsis"></span>
        <button class="btn btn-ghost btn-sm btn-icon" title="复制">${icon("copy", "ic-16")}</button>`;
      $("span", row).textContent = u;
      $("button", row).addEventListener("click", () => copyText(u));
      list.appendChild(row);
    }
  });

  $("#b64-enc").addEventListener("click", () => {
    $("#b64-out").textContent = btoa(String.fromCharCode(...new TextEncoder().encode($("#b64-in").value)));
  });
  $("#b64-dec").addEventListener("click", () => {
    try {
      const bin = atob($("#b64-in").value.trim());
      $("#b64-out").textContent = new TextDecoder().decode(Uint8Array.from(bin, (c) => c.charCodeAt(0)));
    } catch {
      $("#b64-out").textContent = "// 解码失败：不是合法的 Base64";
    }
  });
  $("#url-enc").addEventListener("click", () => { $("#url-out").textContent = encodeURIComponent($("#url-in").value); });
  $("#url-dec").addEventListener("click", () => {
    try { $("#url-out").textContent = decodeURIComponent($("#url-in").value); }
    catch { $("#url-out").textContent = "// 解码失败：不是合法的 URL 编码"; }
  });
}

/* ---------- 正则 ---------- */
function escapeHtml(s) {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

function runRegex() {
  const pattern = $("#re-pattern").value;
  const flags = $("#re-flags").value.includes("g") ? $("#re-flags").value : $("#re-flags").value + "g";
  const text = $("#re-text").value;
  const view = $("#re-view"), body = $("#re-matches");
  let re;
  try { re = new RegExp(pattern, flags); }
  catch (err) {
    view.innerHTML = `<span style="color:var(--danger)">正则无效：${escapeHtml(err.message)}</span>`;
    body.innerHTML = "";
    $("#re-count").textContent = "";
    return;
  }
  const matches = Array.from(text.matchAll(re)).filter((m) => m[0] !== "");
  let html = "", last = 0;
  matches.forEach((m) => {
    html += escapeHtml(text.slice(last, m.index));
    html += `<span class="match-mark">${escapeHtml(m[0])}</span>`;
    last = m.index + m[0].length;
  });
  html += escapeHtml(text.slice(last));
  view.innerHTML = html || '<span class="faint">（空文本）</span>';
  $("#re-count").textContent = `· ${matches.length} 处匹配`;
  body.innerHTML = matches.length
    ? matches.map((m, i) => `<tr><td class="num">${i + 1}</td><td class="mono">${escapeHtml(m[0])}</td><td class="num">${m.index}</td><td class="num">${m[0].length}</td></tr>`).join("")
    : '<tr><td colspan="4" class="muted" style="text-align:center">没有匹配</td></tr>';
}

function initRegex() {
  ["#re-pattern", "#re-flags", "#re-text"].forEach((sel) => $(sel).addEventListener("input", runRegex));
  $$("[data-re]").forEach((chip) => chip.addEventListener("click", () => {
    $("#re-pattern").value = chip.dataset.re;
    runRegex();
  }));
  runRegex();
}

/* ---------- curl 转换 ---------- */
function parseCurl(cmd) {
  const tokens = cmd.match(/'[^']*'|"[^"]*"|\S+/g) || [];
  const clean = tokens.map((t) => t.replace(/^['"]|['"]$/g, ""));
  const req = { url: "", method: "GET", headers: [], data: "" };
  for (let i = 0; i < clean.length; i++) {
    const t = clean[i];
    if (t === "curl") continue;
    if (t === "-X" || t === "--request") req.method = clean[++i];
    else if (t === "-H" || t === "--header") req.headers.push(clean[++i]);
    else if (t === "-d" || t === "--data" || t === "--data-raw") { req.data = clean[++i]; if (req.method === "GET") req.method = "POST"; }
    else if (!t.startsWith("-") && /^https?:\/\//.test(t)) req.url = t;
  }
  return req;
}

function curlToCode(req, lang) {
  const headers = req.headers.map((h) => {
    const idx = h.indexOf(":");
    return [h.slice(0, idx).trim(), h.slice(idx + 1).trim()];
  });
  if (lang === "fetch") {
    const hs = headers.map(([k, v]) => `    "${k}": "${v}",`).join("\n");
    return `const res = await fetch("${req.url}", {\n  method: "${req.method}",\n${hs ? `  headers: {\n${hs}\n  },\n` : ""}${req.data ? `  body: JSON.stringify(${req.data}),\n` : ""});\nconst data = await res.json();`;
  }
  if (lang === "axios") {
    const hs = headers.map(([k, v]) => `    "${k}": "${v}",`).join("\n");
    return `const { data } = await axios({\n  url: "${req.url}",\n  method: "${req.method.toLowerCase()}",\n${hs ? `  headers: {\n${hs}\n  },\n` : ""}${req.data ? `  data: ${req.data},\n` : ""});`;
  }
  const hs = headers.map(([k, v]) => `    "${k}": "${v}",`).join("\n");
  return `import requests\n\nresp = requests.${req.method.toLowerCase()}(\n    "${req.url}",\n${hs ? `    headers={\n${hs}\n    },\n` : ""}${req.data ? `    json=${req.data},\n` : ""})\nprint(resp.json())`;
}

function initCurl() {
  let lang = "fetch";
  $("#curl-target").addEventListener("click", (e) => {
    const btn = e.target.closest("button[data-lang]");
    if (!btn) return;
    lang = btn.dataset.lang;
    $$("#curl-target button").forEach((b) => b.classList.toggle("on", b === btn));
  });
  $("#curl-convert").addEventListener("click", () => {
    const req = parseCurl($("#curl-in").value);
    $("#curl-out").textContent = req.url ? curlToCode(req, lang) : "// 未识别到 URL，请检查 curl 命令";
  });
}

/* ---------- 辅助工具 ---------- */
function genPassword() {
  let pool = "";
  if ($("#pwd-lower").checked) pool += "abcdefghijklmnopqrstuvwxyz";
  if ($("#pwd-upper").checked) pool += "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
  if ($("#pwd-digit").checked) pool += "0123456789";
  if ($("#pwd-symbol").checked) pool += "!@#$%^&*()-_=+[]{}";
  if ($("#pwd-ambiguous").checked) pool = pool.replace(/[0O1lI]/g, "");
  const len = Number($("#pwd-len").value);
  $("#pwd-len-val").textContent = len;
  if (!pool) { $("#pwd-out").textContent = "—"; return; }
  const buf = new Uint32Array(len);
  crypto.getRandomValues(buf);
  const pwd = Array.from(buf, (n) => pool[n % pool.length]).join("");
  $("#pwd-out").textContent = pwd;
  const variety = [$("#pwd-lower").checked, $("#pwd-upper").checked, $("#pwd-digit").checked, $("#pwd-symbol").checked].filter(Boolean).length;
  const pill = $("#pwd-strength");
  const strong = len >= 12 && variety >= 3;
  const mid = len >= 8 && variety >= 2;
  pill.className = `pill ${strong ? "pill-ok" : mid ? "pill-warn" : "pill-err"}`;
  pill.textContent = strong ? "强" : mid ? "中" : "弱";
}

const CASES = [
  ["全部大写", (w) => w.join(" ").toUpperCase()],
  ["全部小写", (w) => w.join(" ").toLowerCase()],
  ["首字母大写", (w) => w.map((x) => x.charAt(0).toUpperCase() + x.slice(1)).join(" ")],
  ["camelCase", (w) => w.map((x, i) => i ? x.charAt(0).toUpperCase() + x.slice(1) : x).join("")],
  ["PascalCase", (w) => w.map((x) => x.charAt(0).toUpperCase() + x.slice(1)).join("")],
  ["snake_case", (w) => w.join("_")],
  ["kebab-case", (w) => w.join("-")],
  ["CONSTANT_CASE", (w) => w.join("_").toUpperCase()],
  ["dot.case", (w) => w.join(".")],
  ["Title Case", (w) => w.map((x) => x.charAt(0).toUpperCase() + x.slice(1)).join(" ")],
];

function wordsOf(text) {
  return text
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .split(/[\s_\-.]+/)
    .filter(Boolean)
    .map((w) => w.toLowerCase());
}

function initAssist() {
  $("#pwd-gen").addEventListener("click", genPassword);
  $("#pwd-len").addEventListener("input", genPassword);
  $$("#pwd-lower, #pwd-upper, #pwd-digit, #pwd-symbol, #pwd-ambiguous").forEach((c) => c.addEventListener("change", genPassword));
  genPassword();

  const grid = $("#case-grid");
  grid.innerHTML = CASES.map(([label], i) => `<button class="btn btn-outline btn-sm" data-case="${i}">${label}</button>`).join("");
  grid.addEventListener("click", (e) => {
    const btn = e.target.closest("[data-case]");
    if (!btn) return;
    const words = wordsOf($("#case-in").value);
    $("#case-out").textContent = words.length ? CASES[Number(btn.dataset.case)][1](words) : "—";
    $$("#case-grid .btn").forEach((b) => {
      b.classList.toggle("btn-primary", b === btn);
      b.classList.toggle("btn-outline", b !== btn);
    });
  });
}

document.addEventListener("DOMContentLoaded", () => {
  initJson();
  initCodec();
  initRegex();
  initCurl();
  initAssist();
});