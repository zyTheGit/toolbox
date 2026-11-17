/**
 * JSON 值 → TypeScript 类型定义。
 * 纯函数、无 DOM 依赖，便于单测。
 *
 * 规则：
 * - 根对象产出 `interface Root`；根为数组/基础类型时产出 `type Root = ...`
 * - 数组元素若全为对象，合并为单个 interface，缺字段标记为可选
 * - 联合类型作为数组元素时补括号：`(A | B)[]`
 * - 命名冲突自动加数字后缀，避免同名 interface 互相覆盖
 */

interface FieldSpec {
  /** 原始属性名，输出时按需加引号 */
  name: string;
  /** 数组中并非每个元素都含该字段 */
  optional: boolean;
  /** 已推断完成的类型表达式 */
  type: string;
}

/** JSON 值转 TypeScript 类型定义；根对象直接产出 interface，其余产出 type 别名。 */
export function jsonToTs(value: unknown, rootName = "Root"): string {
  return new JsonTypeGenerator(rootName).generate(value);
}

class JsonTypeGenerator {
  private readonly rootName: string;
  /** 已占用的类型名，用于冲突后缀 */
  private readonly used = new Set<string>();
  private readonly decls: string[] = [];

  constructor(rootName: string) {
    this.rootName = rootName;
    // 根名先占位，避免子类型抢走 Root
    this.used.add(rootName);
  }

  generate(value: unknown): string {
    let rootType: string;
    if (isRecord(value) && Object.keys(value).length > 0) {
      this.decls.push(this.renderDecl(this.rootName, this.fieldsOf(value, this.rootName)));
      rootType = this.rootName;
    } else {
      rootType = this.infer(value, this.rootName);
    }
    const decls = this.decls.join("\n\n");
    // 根对象已声明为 interface，不能再产出同名的 type 别名
    if (rootType === this.rootName) return decls;
    const alias = `type ${this.rootName} = ${rootType};`;
    return decls ? `${decls}\n\n${alias}` : alias;
  }

  /** 递归推断单个值的类型；对象与「全对象数组」会注册 interface。 */
  private infer(value: unknown, nameHint: string): string {
    if (value === null) return "null";
    if (Array.isArray(value)) return this.arrayType(value, nameHint);
    if (isRecord(value)) {
      if (!Object.keys(value).length) return "Record<string, unknown>";
      return this.intern(nameHint, this.fieldsOf(value, nameHint));
    }
    return typeof value;
  }

  private arrayType(items: unknown[], nameHint: string): string {
    if (!items.length) return "unknown[]";
    // 数组元素取路径末段做单数名：root_users → User，重名时由 intern 加后缀
    const elemHint = singular(lastSegment(nameHint));
    if (items.every(isRecord)) {
      const fields = this.mergedFields(items as Record<string, unknown>[], elemHint);
      return `${this.intern(elemHint, fields)}[]`;
    }
    const union = unionOf(items.map((item) => this.infer(item, elemHint)));
    return union.includes(" | ") ? `(${union})[]` : `${union}[]`;
  }

  private fieldsOf(obj: Record<string, unknown>, nameHint: string): FieldSpec[] {
    return Object.entries(obj).map(([name, child]) => ({
      name,
      optional: false,
      type: this.infer(child, this.childHint(nameHint, name)),
    }));
  }

  /** 拼接子节点命名路径；根节点的子对象不再带 Root 前缀 */
  private childHint(parentHint: string, prop: string): string {
    return parentHint === this.rootName ? prop : `${parentHint}_${prop}`;
  }

  /** 合并对象数组的字段：类型取并集，非全量出现的字段标记可选。 */
  private mergedFields(items: Record<string, unknown>[], nameHint: string): FieldSpec[] {
    const order: string[] = [];
    const acc = new Map<string, { types: string[]; present: number }>();
    for (const item of items) {
      for (const [name, child] of Object.entries(item)) {
        let entry = acc.get(name);
        if (!entry) {
          entry = { types: [], present: 0 };
          acc.set(name, entry);
          order.push(name);
        }
        entry.present += 1;
        entry.types.push(this.infer(child, this.childHint(nameHint, name)));
      }
    }
    return order.map((name) => {
      const entry = acc.get(name)!;
      return {
        name,
        optional: entry.present < items.length,
        type: unionOf(entry.types),
      };
    });
  }

  /** 注册 interface 并返回类型名；重名时追加数字后缀。 */
  private intern(nameHint: string, fields: FieldSpec[]): string {
    const base = toTypeName(nameHint);
    let name = base;
    for (let index = 2; this.used.has(name); index += 1) name = `${base}${index}`;
    this.used.add(name);
    this.decls.push(this.renderDecl(name, fields));
    return name;
  }

  private renderDecl(name: string, fields: FieldSpec[]): string {
    const body = fields
      .map((field) => `  ${quoteKey(field.name)}${field.optional ? "?" : ""}: ${field.type};`)
      .join("\n");
    return `interface ${name} {\n${body}\n}`;
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

const IDENTIFIER = /^[\p{L}_$][\p{L}\p{N}_$]*$/u;
/** 命名分段：按非字母数字切分，下划线也作为分隔符 */
const IDENTIFIER_PART = /[\p{L}\p{N}]+/gu;

/** 非合法标识符的属性名加引号；中文属性名本身是合法标识符，不加引号 */
function quoteKey(name: string): string {
  return IDENTIFIER.test(name) ? name : JSON.stringify(name);
}

/** 路径词转类型名：user_info → UserInfo，用户 → 用户 */
function toTypeName(hint: string): string {
  const words = hint.match(IDENTIFIER_PART) ?? [];
  let name = words.map((word) => word.charAt(0).toUpperCase() + word.slice(1)).join("");
  if (!name) return "Item";
  if (!/^[\p{L}_$]/u.test(name)) name = `T${name}`;
  return name;
}

/** 取路径末段：root_data_items → items */
function lastSegment(nameHint: string): string {
  const parts = nameHint.split("_");
  return parts[parts.length - 1] || nameHint;
}

/** 简单单数化：categories → category，users → user，data → dataItem */
function singular(name: string): string {
  if (/ies$/i.test(name) && name.length > 3) return `${name.slice(0, -3)}y`;
  if (/(ses|xes|zes|ches|shes)$/i.test(name)) return name.slice(0, -2);
  if (/s$/i.test(name) && !/ss$/i.test(name)) return name.slice(0, -1);
  return `${name}Item`;
}

/** 类型并集去重，null 放末尾（更贴近日常写法） */
function unionOf(types: string[]): string {
  let unique = [...new Set(types)];
  if (unique.length > 1) {
    // 空数组推断出的 unknown[] 不参与合并，让具体类型胜出
    if (unique.includes("unknown[]") && unique.some((t) => t !== "unknown[]" && t.endsWith("[]"))) {
      unique = unique.filter((t) => t !== "unknown[]");
    }
    // unknown 表示信息缺失，与具体类型并存时让位（null 不算具体类型）
    if (unique.includes("unknown") && unique.some((t) => t !== "unknown" && t !== "null")) {
      unique = unique.filter((t) => t !== "unknown");
    }
  }
  if (unique.length === 1) return unique[0];
  unique.sort((a, b) => (a === "null" ? 1 : b === "null" ? -1 : 0));
  return unique.join(" | ");
}
