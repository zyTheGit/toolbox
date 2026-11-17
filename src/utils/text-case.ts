/** 文本大小写与命名风格转换。纯函数，便于单测。 */

export type CaseKey =
  | "upper"
  | "lower"
  | "title"
  | "sentence"
  | "swap"
  | "camel"
  | "pascal"
  | "snake"
  | "kebab"
  | "constant";

/** 转换按钮的展示顺序与文案 */
export const CASE_ACTIONS: { key: CaseKey; label: string }[] = [
  { key: "upper", label: "全部大写" },
  { key: "lower", label: "全部小写" },
  { key: "title", label: "单词首字母大写" },
  { key: "sentence", label: "句首大写" },
  { key: "swap", label: "反转大小写" },
  { key: "camel", label: "camelCase" },
  { key: "pascal", label: "PascalCase" },
  { key: "snake", label: "snake_case" },
  { key: "kebab", label: "kebab-case" },
  { key: "constant", label: "CONSTANT_CASE" },
];

export function convertCase(text: string, key: CaseKey): string {
  switch (key) {
    case "upper":
      return text.toUpperCase();
    case "lower":
      return text.toLowerCase();
    case "swap":
      return text.replace(/[a-zA-Z]/g, (char) =>
        char === char.toLowerCase() ? char.toUpperCase() : char.toLowerCase(),
      );
    case "title":
      // 非字母后的首个字母大写，中文等非字母内容原样保留
      return text.replace(/(^|[^A-Za-z])([a-z])/g, (_, prefix: string, char: string) => {
        return prefix + char.toUpperCase();
      });
    case "sentence":
      return text.toLowerCase().replace(
        /(^|[.!?]["')\]]?\s+)([a-z])/g,
        (_, prefix: string, char: string) => prefix + char.toUpperCase(),
      );
    default:
      return convertNaming(text, key);
  }
}

/** 命名风格转换：先按分隔符与驼峰边界切词 */
function convertNaming(text: string, key: Exclude<CaseKey, "upper" | "lower" | "title" | "sentence" | "swap">): string {
  const words = splitWords(text);
  if (!words.length) return text.trim();
  switch (key) {
    case "camel":
      return words[0].toLowerCase() + words.slice(1).map(capitalizeWord).join("");
    case "pascal":
      return words.map(capitalizeWord).join("");
    case "snake":
      return words.map((word) => word.toLowerCase()).join("_");
    case "kebab":
      return words.map((word) => word.toLowerCase()).join("-");
    case "constant":
      return words.map((word) => word.toUpperCase()).join("_");
  }
}

/** 按非字母数字、下划线、驼峰边界切词：userProfileInfo → user profile info */
function splitWords(text: string): string[] {
  return text
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .replace(/([A-Z]+)([A-Z][a-z])/g, "$1 $2")
    .split(/[^A-Za-z0-9]+/)
    .filter(Boolean);
}

function capitalizeWord(word: string): string {
  return word.charAt(0).toUpperCase() + word.slice(1).toLowerCase();
}
