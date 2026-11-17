/**
 * 带行列定位的 JSON 解析。
 * 原生 JSON.parse 只给字符偏移（部分环境连偏移都没有），
 * 这里用最小解析器给出中文描述 + 行/列 + 出错行片段，便于直接定位。
 */

export interface JsonErrorLocation {
  /** 中文错误描述 */
  message: string;
  /** 1 基行号 */
  line: number;
  /** 1 基列号 */
  column: number;
  /** 0 基字符偏移，可用于选中出错位置 */
  position: number;
  /** 出错行 + 指示符，两行文本 */
  snippet: string;
}

export type JsonParseResult =
  | { ok: true; value: unknown }
  | { ok: false; error: JsonErrorLocation };

/** 解析 JSON；失败时返回带行列信息的错误。 */
export function parseJsonWithLocation(text: string): JsonParseResult {
  try {
    return { ok: true, value: parseJsonText(text) };
  } catch (err) {
    if (err instanceof ParseFailure) {
      return { ok: false, error: locateError(text, err.message, err.position) };
    }
    throw err;
  }
}

class ParseFailure extends Error {
  readonly position: number;

  constructor(message: string, position: number) {
    super(message);
    this.position = position;
  }
}

const NUMBER_PATTERN = /-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?/y;

function parseJsonText(text: string): unknown {
  const len = text.length;
  let index = 0;

  // 复制的文本可能带 BOM，忽略它避免首字符报错
  if (text.charCodeAt(0) === 0xfeff) index = 1;

  const fail = (message: string, at: number = index): never => {
    throw new ParseFailure(message, Math.min(Math.max(at, 0), len));
  };

  const skipWhitespace = (): void => {
    while (index < len) {
      const code = text.charCodeAt(index);
      if (code === 0x20 || code === 0x09 || code === 0x0a || code === 0x0d) {
        index += 1;
      } else {
        return;
      }
    }
  };

  const describe = (at: number): string => {
    if (at >= len) return "文件结尾";
    const char = text[at];
    if (char === "\n") return "换行";
    return `'${char}'`;
  };

  /** 报错前先识别注释，避免把注释误报成缺逗号 */
  const failUnexpected = (message: string): never => {
    if (text[index] === "/") fail("JSON 不支持注释，请删除注释后重试");
    return fail(message);
  };

  const parseString = (): string => {
    const start = index;
    index += 1;
    let out = "";
    while (index < len) {
      const char = text[index];
      if (char === '"') {
        index += 1;
        return out;
      }
      if (char === "\\") {
        const esc = text[index + 1];
        if (esc === undefined) break;
        switch (esc) {
          case '"':
          case "\\":
          case "/":
            out += esc;
            break;
          case "b":
            out += "\b";
            break;
          case "f":
            out += "\f";
            break;
          case "n":
            out += "\n";
            break;
          case "r":
            out += "\r";
            break;
          case "t":
            out += "\t";
            break;
          case "u": {
            const hex = text.slice(index + 2, index + 6);
            if (!/^[0-9a-fA-F]{4}$/.test(hex)) fail("\\u 转义必须紧跟 4 位十六进制", index);
            out += String.fromCharCode(Number.parseInt(hex, 16));
            index += 4;
            break;
          }
          default:
            fail(`非法的转义字符 '\\${esc}'`, index);
        }
        index += 2;
        continue;
      }
      if (text.charCodeAt(index) < 0x20) fail("字符串中不能直接包含控制字符（如换行、制表符）");
      out += char;
      index += 1;
    }
    return fail("字符串未闭合：缺少结尾的双引号", start);
  };

  const parseNumber = (): number => {
    NUMBER_PATTERN.lastIndex = index;
    const match = NUMBER_PATTERN.exec(text);
    if (!match) return fail("不是合法的数字");
    index += match[0].length;
    if (index < len) {
      const next = text[index];
      if (next >= "0" && next <= "9") fail("数字格式不正确：JSON 不允许前导零", index);
      if (next === ".") fail("数字格式不正确：小数点后必须紧跟数字", index);
      if (next === "e" || next === "E") fail("数字格式不正确：指数部分不完整", index);
      if (/[A-Za-z_]/.test(next)) fail(`数字后存在多余字符 ${describe(index)}，是否缺少逗号？`, index);
    }
    return Number(match[0]);
  };

  const parseLiteral = (word: string, value: unknown): unknown => {
    if (!text.startsWith(word, index)) {
      fail(`无法识别的值，JSON 只支持 true / false / null / 数字 / 字符串 / 对象 / 数组`);
    }
    index += word.length;
    return value;
  };

  const parseValue = (): unknown => {
    skipWhitespace();
    if (index >= len) fail("意外的结尾：这里需要一个值");
    const char = text[index];
    switch (char) {
      case "{":
        return parseObject();
      case "[":
        return parseArray();
      case '"':
        return parseString();
      case "t":
        return parseLiteral("true", true);
      case "f":
        return parseLiteral("false", false);
      case "n":
        return parseLiteral("null", null);
      case "'":
        return fail("JSON 字符串必须使用双引号");
      case "/":
        return fail("JSON 不支持注释，请删除注释后重试");
      case ",":
        return fail("这里需要一个值，实际遇到 ','（可能存在多余逗号）");
      case "]":
      case "}":
        return fail(`这里需要一个值，实际遇到 '${char}'（可能多了逗号或括号不匹配）`);
      default:
        if (char === "-" || (char >= "0" && char <= "9")) return parseNumber();
        return fail(`无法识别的字符 '${char}'`);
    }
  };

  const parseObject = (): Record<string, unknown> => {
    index += 1;
    const result: Record<string, unknown> = {};
    skipWhitespace();
    if (text[index] === "}") {
      index += 1;
      return result;
    }
    for (;;) {
      skipWhitespace();
      if (text[index] !== '"') {
        if (text[index] === "'") fail("对象的属性名必须使用双引号");
        if (text[index] === "}") fail("对象末尾不允许尾随逗号");
        failUnexpected(`对象的属性名必须使用双引号，实际遇到 ${describe(index)}`);
      }
      const key = parseString();
      skipWhitespace();
      if (text[index] !== ":") fail(`属性名 "${key}" 后缺少 ':'，实际遇到 ${describe(index)}`);
      index += 1;
      result[key] = parseValue();
      skipWhitespace();
      const char = text[index];
      if (char === ",") {
        index += 1;
        skipWhitespace();
        if (text[index] === "}") fail("对象末尾不允许尾随逗号");
        continue;
      }
      if (char === "}") {
        index += 1;
        return result;
      }
      if (index >= len) fail("意外的结尾：对象缺少 '}'");
      failUnexpected(`属性之间缺少逗号，或对象缺少 '}'，实际遇到 ${describe(index)}`);
    }
  };

  const parseArray = (): unknown[] => {
    index += 1;
    const result: unknown[] = [];
    skipWhitespace();
    if (text[index] === "]") {
      index += 1;
      return result;
    }
    for (;;) {
      result.push(parseValue());
      skipWhitespace();
      const char = text[index];
      if (char === ",") {
        index += 1;
        skipWhitespace();
        if (text[index] === "]") fail("数组末尾不允许尾随逗号");
        continue;
      }
      if (char === "]") {
        index += 1;
        return result;
      }
      if (index >= len) fail("意外的结尾：数组缺少 ']'");
      failUnexpected(`数组元素之间缺少逗号，或数组缺少 ']'，实际遇到 ${describe(index)}`);
    }
  };

  skipWhitespace();
  if (index >= len) fail("内容为空，请先粘贴 JSON", len);
  const value = parseValue();
  skipWhitespace();
  if (index < len) fail(`JSON 结束后存在多余内容，实际遇到 ${describe(index)}`);
  return value;
}

/** 把字符偏移折算成行列，并截取出错行作为提示片段 */
function locateError(text: string, message: string, position: number): JsonErrorLocation {
  const pos = Math.min(Math.max(position, 0), text.length);
  let line = 1;
  let lineStart = 0;
  for (let k = 0; k < pos; k += 1) {
    if (text.charCodeAt(k) === 0x0a) {
      line += 1;
      lineStart = k + 1;
    }
  }
  const newline = text.indexOf("\n", lineStart);
  const lineEnd = newline === -1 ? text.length : newline;
  const rawLine = text.slice(lineStart, lineEnd).replace(/\r$/, "");
  const rawPrefix = text.slice(lineStart, pos).replace(/\r$/, "");
  const rendered = rawLine.replace(/\t/g, "  ");
  const caret = rawPrefix.replace(/\t/g, "  ").length;
  return {
    message,
    line,
    column: pos - lineStart + 1,
    position: pos,
    snippet: `${rendered}\n${" ".repeat(Math.min(caret, rendered.length))}^`,
  };
}
