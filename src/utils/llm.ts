import type { LlmConfig, LlmProviderConfig, LlmTemplate } from "../types";

export const BUILTIN_OPENAI_KEY = "translate.openai";

export function defaultLlmConfig(): LlmConfig {
  return {
    providers: [
      provider(
        "openai",
        "OpenAI",
        "https://api.openai.com/v1",
        "gpt-4o-mini",
        BUILTIN_OPENAI_KEY,
      ),
      provider("glm", "智谱 GLM", "https://open.bigmodel.cn/api/paas/v4", "glm-4-flash", ""),
      provider("deepseek", "DeepSeek", "https://api.deepseek.com/v1", "deepseek-chat", ""),
    ],
    templates: [
      {
        id: "template-translate",
        name: "翻译",
        content: "请把下面的文本翻译成{{target}}，保持原意与格式，只输出译文：\n\n{{text}}",
      },
      {
        id: "template-explain",
        name: "代码解释",
        content: "请解释以下{{language}}代码的作用，并指出潜在问题：\n\n{{code}}",
      },
    ],
  };
}

function provider(
  id: string,
  name: string,
  baseUrl: string,
  model: string,
  keyEntryId: string,
): LlmProviderConfig {
  return { id, name, baseUrl, model, keyEntryId, temperature: 0.7 };
}

export function cloneLlmConfig(config: LlmConfig | null | undefined): LlmConfig {
  return JSON.parse(JSON.stringify(config ?? defaultLlmConfig())) as LlmConfig;
}

const VARIABLE_RE = /\{\{\s*([^{}]+?)\s*\}\}/g;

export function templateVariables(content: string): string[] {
  const out: string[] = [];
  for (const match of content.matchAll(VARIABLE_RE)) {
    const name = (match[1] ?? "").trim();
    if (name && !out.includes(name)) out.push(name);
  }
  return out;
}

export function substituteVariables(
  content: string,
  values: Record<string, string>,
): string {
  return content.replace(VARIABLE_RE, (whole, rawName: string) => {
    const value = values[rawName.trim()];
    return value && value.trim() ? value : whole;
  });
}

export function missingVariables(content: string, values: Record<string, string>): string[] {
  return templateVariables(content).filter((name) => !(values[name] ?? "").trim());
}

export function newId(prefix: string): string {
  const random =
    typeof crypto !== "undefined" && "randomUUID" in crypto
      ? crypto.randomUUID()
      : Math.random().toString(36).slice(2);
  return `${prefix}-${random}`;
}

export function defaultTemplate(content = ""): LlmTemplate {
  return { id: newId("template"), name: "新建模板", content };
}
