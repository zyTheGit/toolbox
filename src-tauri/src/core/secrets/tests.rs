use super::*;

fn temp_path() -> PathBuf {
    std::env::temp_dir().join(format!("toolbox-secrets-{}.bin", uuid::Uuid::new_v4()))
}

#[test]
fn patch_is_write_only_and_persists() {
    let path = temp_path();
    let (store, warning) = SecretStore::load(path.clone());
    assert!(warning.is_none());

    store
        .apply_patch(&SecretPatch {
            openai_api_key: Some(" sk-openai ".into()),
            ..SecretPatch::default()
        })
        .unwrap();
    assert!(store.status().openai);
    assert!(!store.status().deepl);

    let (reloaded, warning) = SecretStore::load(path.clone());
    assert!(warning.is_none());
    assert_eq!(reloaded.status(), store.status());
    assert_eq!(reloaded.secret_for(BUILTIN_OPENAI).as_deref(), Some("sk-openai"));

    // None leaves the value, empty string clears it.
    reloaded
        .apply_patch(&SecretPatch {
            openai_api_key: None,
            ..SecretPatch::default()
        })
        .unwrap();
    assert!(reloaded.status().openai);
    reloaded
        .apply_patch(&SecretPatch {
            openai_api_key: Some(String::new()),
            ..SecretPatch::default()
        })
        .unwrap();
    assert!(!reloaded.status().openai);
    assert!(reloaded.list().is_empty());

    let _ = std::fs::remove_file(path);
}

#[test]
fn legacy_file_migrates_to_entries() {
    let path = temp_path();
    let legacy = SecretsV1 {
        openai_api_key: "legacy-openai".into(),
        deepl_api_key: "legacy-deepl".into(),
        baidu_secret_key: String::new(),
        youdao_app_secret: String::new(),
    };
    std::fs::write(&path, codec::encode_legacy(&legacy).unwrap()).unwrap();

    let (store, warning) = SecretStore::load(path.clone());
    assert!(warning.is_none());
    assert_eq!(store.secret_for(BUILTIN_OPENAI).as_deref(), Some("legacy-openai"));
    assert_eq!(store.secret_for(BUILTIN_DEEPL).as_deref(), Some("legacy-deepl"));
    assert_eq!(store.list().len(), 2);

    // The file is rewritten in the new format, so the next load is not legacy.
    let (again, warning) = SecretStore::load(path.clone());
    assert!(warning.is_none());
    assert_eq!(again.status(), store.status());
    let _ = std::fs::remove_file(path);
}

#[test]
fn custom_entry_crud_keeps_secret_when_blank() {
    let path = temp_path();
    let (store, _) = SecretStore::load(path.clone());
    store
        .save_entry(SecretEntryInput {
            name: "智谱".into(),
            service: "glm".into(),
            secret: "glm-secret".into(),
            note: "测试".into(),
            expires_on: "2099-01-01".into(),
            ..SecretEntryInput::default()
        })
        .unwrap();

    let entry = store.list().into_iter().next().unwrap();
    assert!(!entry.builtin);
    assert_eq!(entry.service, "glm");
    assert!(!entry.masked.contains("glm-secret"));
    assert_eq!(store.reveal(&entry.id).unwrap(), "glm-secret");

    store
        .save_entry(SecretEntryInput {
            id: entry.id.clone(),
            name: "智谱 GLM".into(),
            service: "glm".into(),
            secret: String::new(),
            note: "改名".into(),
            expires_on: "2099-01-01".into(),
        })
        .unwrap();
    let updated = store.list().into_iter().next().unwrap();
    assert_eq!(updated.name, "智谱 GLM");
    assert_eq!(store.reveal(&entry.id).unwrap(), "glm-secret");

    store.delete_entry(&entry.id).unwrap();
    assert!(store.list().is_empty());
    let _ = std::fs::remove_file(path);
}

#[test]
fn entry_validation_rejects_bad_expiry_and_empty_secret() {
    let path = temp_path();
    let (store, _) = SecretStore::load(path.clone());
    let bad_date = store.save_entry(SecretEntryInput {
        name: "x".into(),
        service: "x".into(),
        secret: "y".into(),
        expires_on: "2026/01/01".into(),
        ..SecretEntryInput::default()
    });
    assert!(bad_date.is_err());

    let no_secret = store.save_entry(SecretEntryInput {
        name: "x".into(),
        service: "x".into(),
        ..SecretEntryInput::default()
    });
    assert!(no_secret.is_err());
    let _ = std::fs::remove_file(path);
}

#[test]
fn expiring_entries_are_reported() {
    let path = temp_path();
    let (store, _) = SecretStore::load(path.clone());
    let today = Local::now().date_naive();
    store
        .save_entry(SecretEntryInput {
            name: "将过期".into(),
            service: "test".into(),
            secret: "secret-value".into(),
            expires_on: (today + chrono::Duration::days(2)).format("%Y-%m-%d").to_string(),
            ..SecretEntryInput::default()
        })
        .unwrap();
    store
        .save_entry(SecretEntryInput {
            name: "已过期".into(),
            service: "test".into(),
            secret: "secret-value-2".into(),
            expires_on: (today - chrono::Duration::days(3)).format("%Y-%m-%d").to_string(),
            ..SecretEntryInput::default()
        })
        .unwrap();

    let list = store.list();
    let expired = list.iter().find(|entry| entry.name == "已过期").unwrap();
    assert!(expired.expired);
    assert_eq!(expired.days_left, Some(-3));

    let warnings = store.expiring_warnings();
    assert_eq!(warnings.len(), 2);
    assert!(warnings.iter().any(|item| item.contains("已过期 3 天")));
    let _ = std::fs::remove_file(path);
}

#[test]
fn config_file_does_not_contain_secrets() {
    let path = temp_path();
    let (store, _) = SecretStore::load(path.clone());
    store
        .apply_patch(&SecretPatch {
            deepl_api_key: Some("canary-deepl-key".into()),
            ..SecretPatch::default()
        })
        .unwrap();

    let mut config = AppConfig::default();
    config.translate.deepl.api_key = "canary-inline".into();
    assert!(store.migrate_inline(&mut config).unwrap());
    assert!(config.translate.deepl.api_key.is_empty());

    let dir = std::env::temp_dir().join(format!("toolbox-config-{}", uuid::Uuid::new_v4()));
    let paths = crate::state::AppPaths::from_data_dir(dir);
    let mut realized = config.clone();
    store.apply_to(&mut realized);
    config::save(&paths, &realized).unwrap();
    let text = std::fs::read_to_string(&paths.config_file).unwrap();
    assert!(!text.contains("canary-inline"));
    assert!(!text.contains("canary-deepl-key"));
    assert_eq!(realized.translate.deepl.api_key, "canary-inline");

    let _ = std::fs::remove_dir_all(&paths.data_dir);
    let _ = std::fs::remove_file(path);
}

#[test]
fn corrupt_secret_file_is_backed_up() {
    let path = temp_path();
    std::fs::write(&path, "garbage").unwrap();
    let (_store, warning) = SecretStore::load(path.clone());
    assert!(warning.is_some());
    assert!(!path.exists());
}
