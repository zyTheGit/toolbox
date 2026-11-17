//! curl 命令与 fetch / axios / Python 代码互转（纯前端，不访问网络）。

export interface CurlHeader {
  name: string;
  value: string;
}

export interface CurlFormField {
  name: string;
  value: string;
}

export interface CurlRequest {
  method: string;
  url: string;
  headers: CurlHeader[];
  body: string;
  form: CurlFormField[];
  warnings: string[];
}

const VALUE_OPTIONS = new Set([
  "-X", "--request", "-H", "--header", "-d", "--data", "--data-raw", "--data-binary",
  "--data-urlencode", "-F", "--form", "-u", "--user", "-b", "--cookie", "-A",
  "--user-agent", "--url", "--json", "-e", "--referer", "-o", "--output", "-m",
  "--max-time", "--connect-timeout", "-x", "--proxy", "--cacert", "--cert", "--key",
  "-c", "--cookie-jar", "-T", "--upload-file", "-w", "--write-out",
]);

/** 把命令行拆成参数：支持引号、转义与反斜杠换行。 */
export function tokenizeCurl(input: string): string[] {
  const text = input.replace(/\\\r?\n/g, " ");
  const tokens: string[] = [];
  let current = "";
  let has = false;
  let i = 0;

  const flush = () => {
    if (has) {
      tokens.push(current);
      current = "";
      has = false;
    }
  };

  while (i < text.length) {
    const ch = text[i];
    if (/\s/.test(ch)) {
      flush();
      i += 1;
      continue;
    }
    if (ch === "'") {
      has = true;
      i += 1;
      while (i < text.length && text[i] !== "'") {
        current += text[i];
        i += 1;
      }
      if (i >= text.length) throw new Error("单引号未闭合");
      i += 1;
      continue;
    }
    if (ch === '"') {
      has = true;
      i += 1;
      while (i < text.length && text[i] !== '"') {
        if (text[i] === "\\" && i + 1 < text.length && '"\\$`'.includes(text[i + 1])) {
          current += text[i + 1];
          i += 2;
        } else {
          current += text[i];
          i += 1;
        }
      }
      if (i >= text.length) throw new Error("双引号未闭合");
      i += 1;
      continue;
    }
    if (ch === "$" && text[i + 1] === "'") {
      has = true;
      i += 2;
      while (i < text.length && text[i] !== "'") {
        if (text[i] === "\\" && i + 1 < text.length) {
          const next = text[i + 1];
          current += next === "n" ? "\n" : next === "t" ? "\t" : next === "r" ? "\r" : next;
          i += 2;
        } else {
          current += text[i];
          i += 1;
        }
      }
      if (i >= text.length) throw new Error("$' 引号未闭合");
      i += 1;
      continue;
    }
    if (ch === "\\" && i + 1 < text.length) {
      has = true;
      current += text[i + 1];
      i += 2;
      continue;
    }
    has = true;
    current += ch;
    i += 1;
  }
  flush();
  return tokens;
}

export function parseCurl(input: string): CurlRequest {
  const tokens = tokenizeCurl(input.trim());
  if (!tokens.length) throw new Error("请输入 curl 命令");
  if (tokens[0] === "curl") tokens.shift();
  else if (!tokens[0].startsWith("-") && !/^[a-z][a-z0-9+.-]*:\/\//i.test(tokens[0])) {
    throw new Error("命令需以 curl 开头");
  }

  const request: CurlRequest = {
    method: "",
    url: "",
    headers: [],
    body: "",
    form: [],
    warnings: [],
  };
  const candidates: string[] = [];
  let useGet = false;
  let jsonMode = false;

  const setHeader = (name: string, value: string) => {
    const existing = request.headers.find((item) => item.name.toLowerCase() === name.toLowerCase());
    if (existing) existing.value = value;
    else request.headers.push({ name, value });
  };

  for (let i = 0; i < tokens.length; i += 1) {
    const token = tokens[i];
    if (token.startsWith("--") && token.includes("=")) {
      const index = token.indexOf("=");
      handleOption(token.slice(0, index), token.slice(index + 1));
      continue;
    }
    if (VALUE_OPTIONS.has(token)) {
      const value = tokens[i + 1] ?? "";
      i += 1;
      handleOption(token, value);
      continue;
    }
    if (token.startsWith("-") && token.length > 1) {
      handleFlag(token);
      continue;
    }
    candidates.push(token);
  }

  function handleFlag(flag: string): void {
    if (flag === "-G" || flag === "--get") useGet = true;
    else if (flag === "-I" || flag === "--head") request.method = "HEAD";
    else if (flag === "-k" || flag === "--insecure") {
      request.warnings.push("已忽略 -k：示例代码不会跳过证书校验");
    } else if (flag === "--compressed") {
      setHeader("Accept-Encoding", "gzip, deflate, br");
    }
  }

  function handleOption(option: string, value: string): void {
    switch (option) {
      case "-X":
      case "--request":
        request.method = value.trim().toUpperCase();
        break;
      case "-H":
      case "--header": {
        const index = value.indexOf(":");
        if (index >= 0) setHeader(value.slice(0, index).trim(), value.slice(index + 1).trim());
        else if (value.trim()) setHeader(value.trim(), "");
        break;
      }
      case "-d":
      case "--data":
      case "--data-raw":
      case "--data-binary":
      case "--data-urlencode": {
        if (value.startsWith("@")) {
          request.warnings.push(`无法读取本地文件 ${value}，已按字面值输出`);
        }
        request.body = request.body ? `${request.body}&${value}` : value;
        break;
      }
      case "--json":
        request.body = value;
        jsonMode = true;
        break;
      case "-F":
      case "--form": {
        const index = value.indexOf("=");
        const name = index >= 0 ? value.slice(0, index) : value;
        const fieldValue = index >= 0 ? value.slice(index + 1) : "";
        if (fieldValue.startsWith("@")) {
          request.warnings.push(`表单字段 ${name} 引用本地文件，示例中需手动替换`);
        }
        request.form.push({ name, value: fieldValue });
        break;
      }
      case "-u":
      case "--user": {
        const index = value.indexOf(":");
        const user = index >= 0 ? value.slice(0, index) : value;
        const password = index >= 0 ? value.slice(index + 1) : "";
        setHeader("Authorization", `Basic ${toBase64(`${user}:${password}`)}`);
        break;
      }
      case "-b":
      case "--cookie":
        if (value.includes("=")) setHeader("Cookie", value);
        else request.warnings.push(`已忽略 cookie 文件 ${value}`);
        break;
      case "-A":
      case "--user-agent":
        setHeader("User-Agent", value);
        break;
      case "-e":
      case "--referer":
        setHeader("Referer", value);
        break;
      case "--url":
        request.url = value;
        break;
      default:
        break;
    }
  }

  if (!request.url) request.url = candidates.find((item) => /^[a-z][a-z0-9+.-]*:\/\//i.test(item)) ?? candidates[0] ?? "";
  if (!request.url) throw new Error("未找到请求 URL");

  if (useGet) {
    if (request.body) {
      request.url += `${request.url.includes("?") ? "&" : "?"}${request.body}`;
      request.body = "";
    }
    if (!request.method) request.method = "GET";
  }
  if (!request.method) request.method = request.body || request.form.length ? "POST" : "GET";

  if (request.body && !hasHeader(request, "content-type")) {
    setHeader("Content-Type", jsonMode ? "application/json" : "application/x-www-form-urlencoded");
  }
  if (request.form.length) {
    request.warnings.push("multipart 的边界由运行时生成，请勿手动设置 Content-Type");
  }
  return request;
}

export function parseHeadersText(text: string): CurlHeader[] {
  return text
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line && !line.startsWith("#"))
    .map((line) => {
      const index = line.indexOf(":");
      if (index < 0) return { name: line, value: "" };
      return { name: line.slice(0, index).trim(), value: line.slice(index + 1).trim() };
    })
    .filter((header) => header.name);
}

export function toFetch(request: CurlRequest): string {
  const lines: string[] = [];
  if (request.form.length) {
    lines.push("const form = new FormData();");
    for (const field of request.form) {
      lines.push(`form.append(${quote(field.name)}, ${quote(field.value)});`);
    }
    lines.push("");
  }
  lines.push(`const response = await fetch(${quote(request.url)}, {`);
  lines.push(`  method: ${quote(request.method.toUpperCase())},`);
  if (request.headers.length) {
    lines.push("  headers: {");
    for (const header of request.headers) {
      lines.push(`    ${quote(header.name)}: ${quote(header.value)},`);
    }
    lines.push("  },");
  }
  if (request.form.length) lines.push("  body: form,");
  else if (request.body && request.method !== "GET" && request.method !== "HEAD") {
    lines.push(`  body: \`${escapeTemplate(request.body)}\`,`);
  }
  lines.push("});");
  lines.push("const text = await response.text();");
  lines.push("console.log(response.status, text);");
  return lines.join("\n");
}

export function toAxios(request: CurlRequest): string {
  const lines: string[] = [`import axios from "axios";`, ""];
  if (request.form.length) {
    lines.push("const form = new FormData();");
    for (const field of request.form) {
      lines.push(`form.append(${quote(field.name)}, ${quote(field.value)});`);
    }
    lines.push("");
  }
  lines.push("const response = await axios.request({");
  lines.push(`  url: ${quote(request.url)},`);
  lines.push(`  method: ${quote(request.method.toLowerCase())},`);
  if (request.headers.length) {
    lines.push("  headers: {");
    for (const header of request.headers) {
      lines.push(`    ${quote(header.name)}: ${quote(header.value)},`);
    }
    lines.push("  },");
  }
  if (request.form.length) lines.push("  data: form,");
  else if (request.body && request.method !== "GET" && request.method !== "HEAD") {
    lines.push(`  data: \`${escapeTemplate(request.body)}\`,`);
  }
  lines.push("});");
  lines.push("console.log(response.status, response.data);");
  return lines.join("\n");
}

export function toPython(request: CurlRequest): string {
  const lines: string[] = ["import requests", ""];
  if (request.headers.length) {
    lines.push("headers = {");
    for (const header of request.headers) {
      lines.push(`    ${quote(header.name)}: ${quote(header.value)},`);
    }
    lines.push("}", "");
  }
  if (request.form.length) {
    lines.push("data = {");
    for (const field of request.form) {
      lines.push(`    ${quote(field.name)}: ${quote(field.value)},`);
    }
    lines.push("}", "");
  }
  const args = [quote(request.url)];
  if (request.headers.length) args.push("headers=headers");
  if (request.form.length) args.push("data=data");
  else if (request.body && request.method !== "GET" && request.method !== "HEAD") {
    args.push(`data=${quote(request.body)}`);
  }
  const method = request.method.toLowerCase();
  const supported = ["get", "post", "put", "patch", "delete", "head", "options"];
  if (supported.includes(method)) {
    lines.push(`response = requests.${method}(${args.join(", ")})`);
  } else {
    lines.push(`response = requests.request(${quote(request.method.toUpperCase())}, ${args.join(", ")})`);
  }
  lines.push("print(response.status_code, response.text)");
  return lines.join("\n");
}

export function toCurl(request: CurlRequest): string {
  const parts = ["curl"];
  if (request.method && request.method.toUpperCase() !== "GET") {
    parts.push(`-X ${request.method.toUpperCase()}`);
  }
  parts.push(shellQuote(request.url));
  for (const header of request.headers) {
    parts.push(`-H ${shellQuote(`${header.name}: ${header.value}`)}`);
  }
  for (const field of request.form) {
    parts.push(`-F ${shellQuote(`${field.name}=${field.value}`)}`);
  }
  if (!request.form.length && request.body) {
    parts.push(`-d ${shellQuote(request.body)}`);
  }
  return parts.join(" \\\n  ");
}

function hasHeader(request: CurlRequest, name: string): boolean {
  return request.headers.some((header) => header.name.toLowerCase() === name);
}

function quote(value: string): string {
  return JSON.stringify(value).replace(/\u2028/g, "\\u2028").replace(/\u2029/g, "\\u2029");
}

function escapeTemplate(value: string): string {
  return value.replace(/\\/g, "\\\\").replace(/`/g, "\\`").replace(/\$\{/g, "\\${");
}

function shellQuote(value: string): string {
  if (/^[\w@%+=:,./-]+$/.test(value)) return value;
  return `'${value.replace(/'/g, `'\\''`)}'`;
}

function toBase64(text: string): string {
  const bytes = new TextEncoder().encode(text);
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary);
}
