import type { DialogApi } from "naive-ui";

import { api } from "../api";
import { store } from "../store";
import { saveConfigWithConflict } from "./configConflict";
import type { AppConfig } from "../types";

/** 只提交 security.allow_commands 段，冲突时不覆盖其他页面已保存的设置。 */
export async function enableCommandsWithConflict(dialog: DialogApi): Promise<boolean> {
  if (!store.config) return false;
  const payload = JSON.parse(JSON.stringify(store.config)) as AppConfig;
  payload.security.allowCommands = true;
  const result = await saveConfigWithConflict(
    dialog,
    (base) => api.saveConfig(payload, null, "security", base),
    store.configRevision,
    (conflict) => {
      store.config = conflict.config;
      store.configRevision = conflict.configRevision;
    },
  );
  if (!result) return Boolean(store.config?.security.allowCommands);
  store.configRevision = result.configRevision;
  store.config = result.config;
  store.tools = result.tools;
  store.providers = result.providers;
  store.secrets = result.secrets;
  return true;
}
