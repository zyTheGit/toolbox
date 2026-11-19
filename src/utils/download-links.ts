/** 从纯链接或分享文本中提取 http(s) 链接。 */
const URL_PATTERN = /https?:\/\/[\x21-\x7E]+/gi;
const TRAILING_PUNCTUATION = /[)\]}>）】」』》，。、；：！？;,.!?…"'`]+$/u;

export function extractUrls(text: string): string[] {
  const urls: string[] = [];
  for (const match of text.matchAll(URL_PATTERN)) {
    const candidate = match[0].replace(TRAILING_PUNCTUATION, "");
    if (!candidate) continue;
    try {
      const parsed = new URL(candidate);
      if (parsed.protocol === "http:" || parsed.protocol === "https:") {
        urls.push(candidate);
      }
    } catch {
      continue;
    }
  }
  return urls;
}
