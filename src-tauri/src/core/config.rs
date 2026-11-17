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
        return Err("[PARAM] 启用快捷翻译时需要填写快捷键".into());
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

        let mut config: AppConfig = serde_json::from_value(value).unwrap();
        normalize(&mut config, &test_paths());

        assert_eq!(config.schema_version, CONFIG_SCHEMA_VERSION);
        assert!(config.notify.download);
        assert!(config.notify.task);
        assert!(!config.quick_translate.enabled);
        assert!(!config.quick_translate.shortcut.is_empty());
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
}
