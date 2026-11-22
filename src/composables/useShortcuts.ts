import { computed } from "vue";
import { store } from "../store";
import type { ShortcutsConfig } from "../types";
import { DEFAULT_SHORTCUTS, formatShortcut, matchesShortcut, type KeyCombo } from "../utils/shortcut";

export type ShortcutAction = keyof ShortcutsConfig;

/** 页面内快捷键统一读取设置，未加载配置时回退默认值。 */
export function useShortcuts() {
  const platform = computed(() => store.platform || "windows");
  const bindings = computed<ShortcutsConfig>(() => ({
    ...DEFAULT_SHORTCUTS,
    ...(store.config?.shortcuts ?? {}),
  }));

  function label(action: ShortcutAction): string {
    return formatShortcut(bindings.value[action], platform.value);
  }

  function matches(event: KeyCombo & { repeat?: boolean }, action: ShortcutAction): boolean {
    if (event.repeat) return false;
    return matchesShortcut(event, bindings.value[action], platform.value);
  }

  return { bindings, platform, label, matches };
}
