import type { ShortcutsConfig } from "../types";

export type PlatformName = "windows" | "macos" | "linux";

/** 只保留录制所需字段，便于在单测里用普通对象代替 KeyboardEvent。 */
export interface KeyCombo {
  code: string;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
}

const MODIFIER_CODES = new Set([
  "ControlLeft",
  "ControlRight",
  "AltLeft",
  "AltRight",
  "ShiftLeft",
  "ShiftRight",
  "MetaLeft",
  "MetaRight",
]);

/** 与 global-hotkey 解析器支持的非字母数字键保持一致。 */
const NAMED_KEYS = new Set([
  "Backquote",
  "Backslash",
  "BracketLeft",
  "BracketRight",
  "Pause",
  "Comma",
  "Equal",
  "Minus",
  "Period",
  "Quote",
  "Semicolon",
  "Slash",
  "Backspace",
  "CapsLock",
  "Enter",
  "Space",
  "Tab",
  "Delete",
  "End",
  "Home",
  "Insert",
  "PageDown",
  "PageUp",
  "PrintScreen",
  "ScrollLock",
  "ArrowDown",
  "ArrowLeft",
  "ArrowRight",
  "ArrowUp",
  "NumLock",
  "NumpadAdd",
  "NumpadDecimal",
  "NumpadDivide",
  "NumpadEnter",
  "NumpadEqual",
  "NumpadMultiply",
  "NumpadSubtract",
  "Escape",
  "AudioVolumeDown",
  "AudioVolumeUp",
  "AudioVolumeMute",
  "MediaPlayPause",
  "MediaStop",
  "MediaTrackNext",
  "MediaTrackPrevious",
]);

export function normalizePlatform(platform: string): PlatformName {
  const value = platform.toLowerCase();
  if (value === "macos" || value === "darwin" || value === "mac") return "macos";
  if (value === "linux") return "linux";
  return "windows";
}

export function isModifierCode(code: string): boolean {
  return MODIFIER_CODES.has(code);
}

export function hasModifier(combo: KeyCombo): boolean {
  return combo.ctrlKey || combo.altKey || combo.shiftKey || combo.metaKey;
}

function isSupportedKey(code: string): boolean {
  return (
    /^Key[A-Z]$/.test(code) ||
    /^Digit[0-9]$/.test(code) ||
    /^Numpad[0-9]$/.test(code) ||
    /^F([1-9]|1[0-9]|2[0-4])$/.test(code) ||
    NAMED_KEYS.has(code)
  );
}

export function modifierTokens(combo: KeyCombo, platform: string): string[] {
  const os = normalizePlatform(platform);
  const tokens: string[] = [];
  if (os === "macos") {
    if (combo.metaKey) tokens.push("CommandOrControl");
    if (combo.ctrlKey) tokens.push("Control");
  } else {
    if (combo.ctrlKey) tokens.push("CommandOrControl");
    if (combo.metaKey) tokens.push("Super");
  }
  if (combo.altKey) tokens.push("Alt");
  if (combo.shiftKey) tokens.push("Shift");
  return tokens;
}

/**
 * 把一次按键转成 global-hotkey 可解析的快捷键字符串。
 * 没有修饰键或按键不被支持时返回 null，避免注册会抢占全局输入的单键。
 */
export function acceleratorFromCombo(combo: KeyCombo, platform: string): string | null {
  if (!isSupportedKey(combo.code)) return null;
  const modifiers = modifierTokens(combo, platform);
  if (!modifiers.length) return null;
  return [...modifiers, combo.code].join("+");
}

function modifierLabel(token: string, os: PlatformName): string | null {
  switch (token.toUpperCase()) {
    case "COMMANDORCONTROL":
    case "COMMANDORCTRL":
    case "CMDORCTRL":
    case "CMDORCONTROL":
      return os === "macos" ? "Cmd" : "Ctrl";
    case "CONTROL":
    case "CTRL":
      return "Ctrl";
    case "ALT":
    case "OPTION":
      return os === "macos" ? "Option" : "Alt";
    case "SHIFT":
      return "Shift";
    case "COMMAND":
    case "CMD":
    case "META":
    case "SUPER":
      return os === "windows" ? "Win" : os === "macos" ? "Cmd" : "Super";
    default:
      return null;
  }
}

const KEY_LABELS: Record<string, string> = {
  ARROWDOWN: "↓",
  ARROWLEFT: "←",
  ARROWRIGHT: "→",
  ARROWUP: "↑",
  SPACE: "空格",
  ESCAPE: "Esc",
  BACKSPACE: "退格",
  DELETE: "Del",
  INSERT: "Ins",
  PAGEUP: "PgUp",
  PAGEDOWN: "PgDn",
  CAPSLOCK: "Caps",
  NUMLOCK: "Num",
  SCROLLLOCK: "Scroll",
  PRINTSCREEN: "PrtSc",
  BACKQUOTE: "`",
  MINUS: "-",
  EQUAL: "=",
  BRACKETLEFT: "[",
  BRACKETRIGHT: "]",
  BACKSLASH: "\\",
  SEMICOLON: ";",
  QUOTE: "'",
  COMMA: ",",
  PERIOD: ".",
  SLASH: "/",
  NUMPADADD: "Num+",
  NUMPADSUBTRACT: "Num-",
  NUMPADMULTIPLY: "Num*",
  NUMPADDIVIDE: "Num/",
  NUMPADDECIMAL: "Num.",
  NUMPADENTER: "NumEnter",
  NUMPADEQUAL: "Num=",
  AUDIOVOLUMEUP: "音量+",
  AUDIOVOLUMEDOWN: "音量-",
  AUDIOVOLUMEMUTE: "静音",
  MEDIAPLAYPAUSE: "播放/暂停",
  MEDIASTOP: "停止",
  MEDIATRACKNEXT: "下一曲",
  MEDIATRACKPREVIOUS: "上一曲",
};

function keyLabel(token: string): string {
  const upper = token.toUpperCase();
  if (/^KEY[A-Z]$/.test(upper)) return upper.slice(3);
  if (/^DIGIT[0-9]$/.test(upper)) return upper.slice(5);
  if (/^NUMPAD[0-9]$/.test(upper)) return `Num${upper.slice(6)}`;
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(upper)) return upper;
  return KEY_LABELS[upper] ?? token;
}

/** 把配置里的快捷键字符串转成界面展示文本。 */
export function formatShortcut(accelerator: string, platform: string): string {
  const os = normalizePlatform(platform);
  const tokens = accelerator
    .split("+")
    .map((token) => token.trim())
    .filter(Boolean);
  return tokens.map((token) => modifierLabel(token, os) ?? keyLabel(token)).join(" + ");
}

/** 录制过程中只按下了修饰键时的实时预览。 */
export function formatModifierPreview(combo: KeyCombo, platform: string): string {
  const os = normalizePlatform(platform);
  return modifierTokens(combo, platform)
    .map((token) => modifierLabel(token, os) ?? token)
    .join(" + ");
}

/** 页面内快捷键的默认值与后端 ShortcutsConfig 保持一致。 */
export const DEFAULT_SHORTCUTS: ShortcutsConfig = {
  translateSubmit: "CommandOrControl+Enter",
  dbRun: "CommandOrControl+Enter",
};

const MODIFIER_CANONICAL: Record<string, "primary" | "control" | "alt" | "shift" | "super"> = {
  COMMANDORCONTROL: "primary",
  COMMANDORCTRL: "primary",
  CMDORCTRL: "primary",
  CMDORCONTROL: "primary",
  CONTROL: "control",
  CTRL: "control",
  ALT: "alt",
  OPTION: "alt",
  SHIFT: "shift",
  COMMAND: "super",
  CMD: "super",
  META: "super",
  SUPER: "super",
};

const KEY_ALIASES: Record<string, string> = {
  "`": "Backquote",
  "\\": "Backslash",
  "[": "BracketLeft",
  "]": "BracketRight",
  PAUSEBREAK: "Pause",
  ",": "Comma",
  "=": "Equal",
  "-": "Minus",
  ".": "Period",
  "'": "Quote",
  ";": "Semicolon",
  "/": "Slash",
  UP: "ArrowUp",
  DOWN: "ArrowDown",
  LEFT: "ArrowLeft",
  RIGHT: "ArrowRight",
  ESC: "Escape",
  DEL: "Delete",
  INS: "Insert",
  RETURN: "Enter",
  NUMPLUS: "NumpadAdd",
  NUMMINUS: "NumpadSubtract",
  NUMSTAR: "NumpadMultiply",
  NUMSLASH: "NumpadDivide",
};

for (const name of NAMED_KEYS) KEY_ALIASES[name.toUpperCase()] = name;
for (let index = 0; index <= 9; index += 1) {
  KEY_ALIASES[`NUM${index}`] = `Numpad${index}`;
}

function canonicalKey(token: string): string | null {
  const upper = token.toUpperCase();
  if (/^KEY[A-Z]$/.test(upper)) return `Key${upper.slice(3)}`;
  if (/^[A-Z]$/.test(upper)) return `Key${upper}`;
  if (/^DIGIT[0-9]$/.test(upper)) return `Digit${upper.slice(5)}`;
  if (/^[0-9]$/.test(upper)) return `Digit${upper}`;
  if (/^NUMPAD[0-9]$/.test(upper)) return `Numpad${upper.slice(6)}`;
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(upper)) return upper;
  return KEY_ALIASES[upper] ?? null;
}

/**
 * 把不同写法的快捷键规整成可比较的形式：修饰键顺序无关，
 * CommandOrControl 按平台折算为物理 Ctrl/Cmd，字母与符号使用物理键名。
 */
export function canonicalAccelerator(accelerator: string, platform: string): string | null {
  const os = normalizePlatform(platform);
  const tokens = accelerator
    .split("+")
    .map((token) => token.trim())
    .filter(Boolean);
  if (!tokens.length) return null;

  const modifiers = new Set<string>();
  let key: string | null = null;
  for (const token of tokens) {
    const modifier = MODIFIER_CANONICAL[token.toUpperCase()];
    if (modifier) {
      const physical = modifier === "primary" ? (os === "macos" ? "super" : "control") : modifier;
      modifiers.add(physical);
      continue;
    }
    const parsed = canonicalKey(token);
    if (!parsed || key) return null;
    key = parsed;
  }
  if (!key) return null;

  const ordered = ["control", "alt", "shift", "super"].filter((item) => modifiers.has(item));
  return [...ordered, key].join("+");
}

/** 判断一次按键是否命中配置的快捷键，只比较语义等价的形式。 */
export function matchesShortcut(combo: KeyCombo, accelerator: string, platform: string): boolean {
  if (!accelerator.trim()) return false;
  const recorded = acceleratorFromCombo(combo, platform);
  if (!recorded) return false;
  const expected = canonicalAccelerator(recorded, platform);
  const configured = canonicalAccelerator(accelerator, platform);
  return expected !== null && expected === configured;
}
