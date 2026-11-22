use std::path::Path;

use crate::core::persist;
use crate::models::{AppConfig, CONFIG_SCHEMA_VERSION};
use crate::state::AppPaths;

/// Returns the config plus an optional warning when the on-disk file could not
/// be read. A corrupt file is backed up by `persist::load_json`, never treated
/// as empty and overwritten.
pub fn load(paths: &AppPaths) -> (AppConfig, Option<String>) {
    let (mut config, warning) = match persist::load_json::<AppConfig>(&paths.config_file) {
        Ok(Some(config)) => (config, None),
        Ok(None) => (AppConfig::default(), None),
        Err(err) => (AppConfig::default(), Some(err)),
    };
    normalize(&mut config, paths);
    (config, warning)
}

pub fn save(paths: &AppPaths, config: &AppConfig) -> Result<(), String> {
    paths
        .ensure_dirs()
        .map_err(|e| format!("创建数据目录失败: {e}"))?;
    let mut stored = config.clone();
    clear_secrets(&mut stored);
    persist::save_json(&paths.config_file, &stored)
}

/// Secrets never belong in config.json; they live in the DPAPI-protected
/// secret store. Clearing before every write is the last line of defence.
pub fn clear_secrets(config: &mut AppConfig) {
    config.translate.openai.api_key.clear();
    config.translate.deepl.api_key.clear();
    config.translate.baidu.secret_key.clear();
    config.translate.youdao.app_secret.clear();
}

/// 保存范围：不同页面只提交自己负责的字段。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigSection {
    /// 除 `llm` 外的全部设置。
    Settings,
    /// 仅 `llm`。
    Llm,
    /// 仅 `security.allow_commands`（任务弹窗开启命令执行时使用）。
    Security,
    /// 整包替换，仅用于兼容旧调用。
    All,
}

impl ConfigSection {
    pub fn parse(value: Option<&str>) -> Result<Self, String> {
        match value.unwrap_or("all").trim() {
            "settings" => Ok(Self::Settings),
            "llm" => Ok(Self::Llm),
            "security" => Ok(Self::Security),
            "all" => Ok(Self::All),
            other => Err(format!("[PARAM] 未知配置保存范围: {other}")),
        }
    }
}

/// 按范围合并配置：未提交的字段保留当前值，避免跨页旧草稿覆盖。
pub fn merge_section(current: &AppConfig, incoming: AppConfig, section: ConfigSection) -> AppConfig {
    match section {
        ConfigSection::All => incoming,
        ConfigSection::Settings => AppConfig {
            llm: current.llm.clone(),
            ..incoming
        },
        ConfigSection::Llm => AppConfig {
            llm: incoming.llm,
            ..current.clone()
        },
        ConfigSection::Security => AppConfig {
            security: crate::models::SecurityConfig {
                allow_commands: incoming.security.allow_commands,
            },
            ..current.clone()
        },
    }
}

pub fn normalize(config: &mut AppConfig, paths: &AppPaths) {
    if config.download_dir.trim().is_empty() {
        config.download_dir = paths.downloads_dir.to_string_lossy().to_string();
    }
    config.download_dir = expand_home(config.download_dir.trim());
    if config.history.max_entries == 0 {
        config.history.max_entries = 200;
    }
    config.history.max_entries = config.history.max_entries.clamp(1, 2000);
    if config.download_concurrency == 0 {
        config.download_concurrency = 2;
    }
    config.download_concurrency = config.download_concurrency.clamp(1, 5);
    if config.fragment_threads == 0 {
        config.fragment_threads = 4;
    }
    config.fragment_threads = config.fragment_threads.clamp(1, 16);
    config.startup.delay_seconds = config.startup.delay_seconds.clamp(0, 300);
    if !config.translate.tts_rate.is_finite() {
        config.translate.tts_rate = 1.0;
    }
    config.translate.tts_rate = config.translate.tts_rate.clamp(0.5, 2.0);
    config.quick_translate.shortcut = config.quick_translate.shortcut.trim().to_string();
    let shortcut_defaults = crate::models::ShortcutsConfig::default();
    config.shortcuts.translate_submit = config.shortcuts.translate_submit.trim().to_string();
    config.shortcuts.db_run = config.shortcuts.db_run.trim().to_string();
    if config.shortcuts.translate_submit.is_empty() {
        config.shortcuts.translate_submit = shortcut_defaults.translate_submit;
    }
    if config.shortcuts.db_run.is_empty() {
        config.shortcuts.db_run = shortcut_defaults.db_run;
    }
    for provider in &mut config.llm.providers {
        provider.id = provider.id.trim().to_string();
        provider.name = provider.name.trim().to_string();
        provider.base_url = provider.base_url.trim().to_string();
        provider.model = provider.model.trim().to_string();
        provider.key_entry_id = provider.key_entry_id.trim().to_string();
        if !provider.temperature.is_finite() {
            provider.temperature = 0.7;
        }
        provider.temperature = provider.temperature.clamp(0.0, 2.0);
    }
    for template in &mut config.llm.templates {
        template.id = template.id.trim().to_string();
        template.name = template.name.trim().to_string();
    }
    if config.schema_version == 0 || config.schema_version < CONFIG_SCHEMA_VERSION {
        config.schema_version = CONFIG_SCHEMA_VERSION;
    }
}

pub fn validate(config: &AppConfig) -> Result<(), String> {
    const PROVIDERS: [&str; 6] = ["microsoft", "google", "openai", "deepl", "baidu", "youdao"];
    if config.schema_version > CONFIG_SCHEMA_VERSION {
        return Err(format!(
            "[DATA] 配置版本 {} 高于当前支持的 {}，请升级 Toolbox 后再保存",
            config.schema_version, CONFIG_SCHEMA_VERSION
        ));
    }
    if !PROVIDERS.contains(&config.translate.provider.as_str()) {
        return Err(format!("[PARAM] 未知翻译服务: {}", config.translate.provider));
    }
    for item in &config.translate.fallback {
        if !PROVIDERS.contains(&item.as_str()) {
            return Err(format!("[PARAM] 未知回退翻译服务: {item}"));
        }
    }
    if config.history.max_entries == 0 {
        return Err("[PARAM] 历史保留条数至少为 1".into());
    }
    if config.quick_translate.enabled && config.quick_translate.shortcut.is_empty() {
        return Err("[HOTKEY] 启用快捷翻译时需要填写快捷键".into());
    }
    let mut provider_ids = std::collections::HashSet::new();
    for provider in &config.llm.providers {
        if provider.id.is_empty() || provider.name.is_empty() {
            return Err("[PARAM] 模型配置缺少 ID 或名称".into());
        }
        if !provider_ids.insert(provider.id.as_str()) {
            return Err(format!("[PARAM] 模型配置 ID 重复: {}", provider.id));
        }
        if provider.model.is_empty() {
            return Err(format!("[PARAM] 模型「{}」缺少模型名", provider.name));
        }
        if !provider.base_url.starts_with("http://") && !provider.base_url.starts_with("https://") {
            return Err(format!("[PARAM] 模型「{}」的接口地址需以 http(s):// 开头", provider.name));
        }
    }
    let mut template_ids = std::collections::HashSet::new();
    for template in &config.llm.templates {
        if template.id.is_empty() || template.name.is_empty() {
            return Err("[PARAM] Prompt 模板缺少 ID 或名称".into());
        }
        if !template_ids.insert(template.id.as_str()) {
            return Err(format!("[PARAM] Prompt 模板 ID 重复: {}", template.id));
        }
        if template.content.trim().is_empty() {
            return Err(format!("[PARAM] Prompt 模板「{}」内容为空", template.name));
        }
    }
    Ok(())
}

pub fn expand_home(value: &str) -> String {
    if value == "~" || value.starts_with("~/") || value.starts_with("~\\") {
        if let Some(home) = home_dir() {
            let rest = value.trim_start_matches('~').trim_start_matches(['/', '\\']);
            return home.join(rest).to_string_lossy().to_string();
        }
    }
    value.to_string()
}

pub fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(std::path::PathBuf::from)
        .filter(|path| Path::new(path).is_absolute())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppPaths;
    use std::path::PathBuf;

    fn test_paths() -> AppPaths {
        AppPaths::from_data_dir(PathBuf::from("C:/tmp/toolbox-config-test"))
    }

    #[test]
    fn tts_rate_is_clamped() {
        let paths = test_paths();
        let mut config = AppConfig::default();

        config.translate.tts_rate = f64::NAN;
        normalize(&mut config, &paths);
        assert_eq!(config.translate.tts_rate, 1.0);

        config.translate.tts_rate = 9.0;
        normalize(&mut config, &paths);
        assert_eq!(config.translate.tts_rate, 2.0);

        config.translate.tts_rate = 0.1;
        normalize(&mut config, &paths);
        assert_eq!(config.translate.tts_rate, 0.5);
    }

    #[test]
    fn old_translate_config_defaults_tts_rate() {
        let json = r#"{
            "provider": "microsoft",
            "fallback": [],
            "openai": { "baseUrl": "", "apiKey": "", "model": "" },
            "deepl": { "apiKey": "", "endpoint": "" },
            "baidu": { "appId": "", "secretKey": "" },
            "youdao": { "appKey": "", "appSecret": "" }
        }"#;
        let config: crate::models::TranslateConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.tts_rate, 1.0);
    }

    #[test]
    fn legacy_config_migrates_new_fields() {
        let mut value = serde_json::to_value(AppConfig::default()).unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("schemaVersion");
        object.remove("notify");
        object.remove("quickTranslate");
        object.remove("shortcuts");

        let mut config: AppConfig = serde_json::from_value(value).unwrap();
        normalize(&mut config, &test_paths());

        assert_eq!(config.schema_version, CONFIG_SCHEMA_VERSION);
        assert!(config.notify.download);
        assert!(config.notify.task);
        assert!(!config.quick_translate.enabled);
        assert!(!config.quick_translate.shortcut.is_empty());
        assert_eq!(config.shortcuts.translate_submit, "CommandOrControl+Enter");
        assert_eq!(config.shortcuts.db_run, "CommandOrControl+Enter");
    }

    #[test]
    fn legacy_notify_config_gets_event_defaults() {
        let mut value = serde_json::to_value(AppConfig::default()).unwrap();
        value["notify"] = serde_json::json!({ "download": true, "task": true });

        let config: AppConfig = serde_json::from_value(value.clone()).unwrap();
        assert!(config.notify.events.success);
        assert!(config.notify.events.failed);
        assert!(!config.notify.events.all_done);
        assert!(!config.notify.sound);

        // 只写了部分事件字段时，其余字段走默认值而不是解析失败。
        value["notify"] = serde_json::json!({
            "download": true,
            "task": true,
            "events": { "success": false }
        });
        let config: AppConfig = serde_json::from_value(value).unwrap();
        assert!(!config.notify.events.success);
        assert!(config.notify.events.failed);
        assert!(!config.notify.events.all_done);
    }

    #[test]
    fn blank_page_shortcuts_fall_back_to_defaults() {
        let mut config = AppConfig::default();
        config.shortcuts.translate_submit = "  ".into();
        config.shortcuts.db_run = "".into();
        normalize(&mut config, &test_paths());
        assert_eq!(config.shortcuts.translate_submit, "CommandOrControl+Enter");
        assert_eq!(config.shortcuts.db_run, "CommandOrControl+Enter");
    }

    #[test]
    fn unknown_config_field_is_rejected() {
        let mut value = serde_json::to_value(AppConfig::default()).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("surprise".into(), serde_json::json!(1));
        assert!(serde_json::from_value::<AppConfig>(value).is_err());
    }

    #[test]
    fn future_schema_version_is_rejected() {
        let mut config = AppConfig::default();
        normalize(&mut config, &test_paths());
        config.schema_version = CONFIG_SCHEMA_VERSION + 1;
        assert!(validate(&config).is_err());
    }

    #[test]
    fn settings_section_preserves_current_llm() {
        let current = AppConfig::default();
        let mut incoming = current.clone();
        incoming.download_dir = "/tmp/new-downloads".into();
        incoming.llm.providers[0].model = "stale-model".into();

        let merged = merge_section(&current, incoming, ConfigSection::Settings);
        assert_eq!(merged.download_dir, "/tmp/new-downloads");
        assert_eq!(
            merged.llm.providers[0].model,
            current.llm.providers[0].model,
            "设置页不得覆盖 AI 页保存的模型配置"
        );
    }

    #[test]
    fn llm_section_preserves_other_settings() {
        let current = AppConfig::default();
        let mut incoming = current.clone();
        incoming.download_dir = "/tmp/stale".into();
        incoming.llm.providers[0].model = "new-model".into();

        let merged = merge_section(&current, incoming, ConfigSection::Llm);
        assert_eq!(merged.download_dir, current.download_dir);
        assert_eq!(merged.llm.providers[0].model, "new-model");
    }

    #[test]
    fn security_section_only_changes_command_switch() {
        let mut current = AppConfig::default();
        current.security.allow_commands = false;
        current.download_dir = "/tmp/current".into();
        current.llm.providers[0].model = "current-model".into();
        let mut incoming = current.clone();
        incoming.security.allow_commands = true;
        incoming.download_dir = "/tmp/stale".into();
        incoming.llm.providers[0].model = "stale-model".into();

        let merged = merge_section(&current, incoming, ConfigSection::Security);
        assert!(merged.security.allow_commands);
        assert_eq!(merged.download_dir, "/tmp/current");
        assert_eq!(merged.llm.providers[0].model, "current-model");
    }

    #[test]
    fn section_names_are_strict() {
        assert_eq!(ConfigSection::parse(Some("settings")).unwrap(), ConfigSection::Settings);
        assert_eq!(ConfigSection::parse(Some("llm")).unwrap(), ConfigSection::Llm);
        assert_eq!(ConfigSection::parse(Some("security")).unwrap(), ConfigSection::Security);
        assert_eq!(ConfigSection::parse(None).unwrap(), ConfigSection::All);
        assert!(ConfigSection::parse(Some("unknown")).is_err());
    }
}
