/** 轻量语法着色：只分词不拼 HTML，渲染时逐个 span，避免注入。 */

export type CodeTokenKind = "plain" | "string" | "comment" | "keyword" | "number";

export interface CodeToken {
  text: string;
  kind: CodeTokenKind;
}

const TS_KEYWORDS = new Set([
  "const", "let", "var", "await", "async", "import", "from", "new", "return",
  "function", "if", "else", "for", "of", "class", "try", "catch", "export",
]);

const PY_KEYWORDS = new Set([
  "import", "from", "def", "return", "print", "True", "False", "None", "with",
  "as", "for", "in", "if", "else", "try", "except",
]);

export function highlightCode(code: string, language: "ts" | "python"): CodeToken[] {
  const keywords = language === "python" ? PY_KEYWORDS : TS_KEYWORDS;
  const pattern =
    language === "python"
      ? /("""[\s\S]*?"""|'''[\s\S]*?'''|"(?:\\[\s\S]|[^"\\])*"|'(?:\\[\s\S]|[^'\\])*')|(#[^\n]*)|(\b\d+(?:\.\d+)?\b)|([A-Za-z_]\w*)/g
      : /(`(?:\\[\s\S]|[^`\\])*`|"(?:\\[\s\S]|[^"\\])*"|'(?:\\[\s\S]|[^'\\])*')|(\/\/[^\n]*)|(\b\d+(?:\.\d+)?\b)|([A-Za-z_$][\w$]*)/g;

  const tokens: CodeToken[] = [];
  let last = 0;
  for (const match of code.matchAll(pattern)) {
    const index = match.index ?? 0;
    if (index > last) tokens.push({ text: code.slice(last, index), kind: "plain" });
    const text = match[0];
    let kind: CodeTokenKind = "plain";
    if (match[1]) kind = "string";
    else if (match[2]) kind = "comment";
    else if (match[3]) kind = "number";
    else if (keywords.has(text)) kind = "keyword";
    tokens.push({ text, kind });
    last = index + text.length;
  }
  if (last < code.length) tokens.push({ text: code.slice(last), kind: "plain" });
  return tokens;
}
