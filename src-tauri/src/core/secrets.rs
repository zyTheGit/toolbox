use std::path::PathBuf;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::core::config;
use crate::core::persist;
use crate::models::AppConfig;

const MAGIC: &[u8] = b"TOOLBOX-SECRETS-1\n";

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Secrets {
    pub openai_api_key: String,
    pub deepl_api_key: String,
    pub baidu_secret_key: String,
    pub youdao_app_secret: String,
}

/// Write-only update for stored secrets. `None` keeps the current value,
/// `Some("")` clears it, any other value replaces it.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SecretPatch {
    pub openai_api_key: Option<String>,
    pub deepl_api_key: Option<String>,
    pub baidu_secret_key: Option<String>,
    pub youdao_app_secret: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SecretStatus {
    pub openai: bool,
    pub deepl: bool,
    pub baidu: bool,
    pub youdao: bool,
}

pub struct SecretStore {
    path: PathBuf,
    secrets: Mutex<Secrets>,
}

impl SecretStore {
    pub fn load(path: PathBuf) -> (Self, Option<String>) {
        let mut warning = None;
        let secrets = match persist::load_bytes(&path) {
            Ok(Some(bytes)) => match decode(&bytes) {
                Ok(secrets) => secrets,
                Err(err) => {
                    warning = Some(match persist::backup_corrupt(&path) {
                        Some(backup) => format!(
                            "读取密钥文件失败（{err}），原文件已备份为 {}",
                            backup.display()
                        ),
                        None => format!("读取密钥文件失败（{err}）"),
                    });
                    Secrets::default()
                }
            },
            Ok(None) => Secrets::default(),
            Err(err) => {
                warning = Some(err);
                Secrets::default()
            }
        };
        (
            Self {
                path,
                secrets: Mutex::new(secrets),
            },
            warning,
        )
    }

    pub fn status(&self) -> SecretStatus {
        let secrets = self.secrets.lock();
        SecretStatus {
            openai: !secrets.openai_api_key.trim().is_empty(),
            deepl: !secrets.deepl_api_key.trim().is_empty(),
            baidu: !secrets.baidu_secret_key.trim().is_empty(),
            youdao: !secrets.youdao_app_secret.trim().is_empty(),
        }
    }

    pub fn apply_to(&self, config: &mut AppConfig) {
        let secrets = self.secrets.lock();
        config.translate.openai.api_key = secrets.openai_api_key.clone();
        config.translate.deepl.api_key = secrets.deepl_api_key.clone();
        config.translate.baidu.secret_key = secrets.baidu_secret_key.clone();
        config.translate.youdao.app_secret = secrets.youdao_app_secret.clone();
    }

    /// Move keys embedded in an older config.json into the secret store and
    /// blank them in the config. Returns whether anything migrated.
    pub fn migrate_inline(&self, config: &mut AppConfig) -> Result<bool, String> {
        let patch = SecretPatch {
            openai_api_key: take_secret(&mut config.translate.openai.api_key),
            deepl_api_key: take_secret(&mut config.translate.deepl.api_key),
            baidu_secret_key: take_secret(&mut config.translate.baidu.secret_key),
            youdao_app_secret: take_secret(&mut config.translate.youdao.app_secret),
        };
        let changed = !patch_is_empty(&patch);
        if changed {
            self.apply_patch(&patch)?;
        }
        Ok(changed)
    }

    pub fn apply_patch(&self, patch: &SecretPatch) -> Result<(), String> {
        let mut secrets = self.secrets.lock();
        let previous = secrets.clone();
        apply_field(&mut secrets.openai_api_key, patch.openai_api_key.as_deref());
        apply_field(&mut secrets.deepl_api_key, patch.deepl_api_key.as_deref());
        apply_field(&mut secrets.baidu_secret_key, patch.baidu_secret_key.as_deref());
        apply_field(&mut secrets.youdao_app_secret, patch.youdao_app_secret.as_deref());
        if *secrets == previous {
            return Ok(());
        }
        let encoded = match encode(&secrets) {
            Ok(encoded) => encoded,
            Err(err) => {
                *secrets = previous;
                return Err(format!("[DATA] 保存密钥失败: {err}"));
            }
        };
        if let Err(err) = persist::write_bytes(&self.path, &encoded) {
            *secrets = previous;
            return Err(format!("[DATA] 保存密钥失败: {err}"));
        }
        Ok(())
    }
}

pub fn mask_config(config: &mut AppConfig) {
    config::clear_secrets(config);
}

fn apply_field(target: &mut String, value: Option<&str>) {
    if let Some(value) = value {
        let value = value.trim().to_string();
        if *target != value {
            *target = value;
        }
    }
}

fn take_secret(value: &mut String) -> Option<String> {
    let taken = value.trim().to_string();
    value.clear();
    if taken.is_empty() {
        None
    } else {
        Some(taken)
    }
}

fn patch_is_empty(patch: &SecretPatch) -> bool {
    patch.openai_api_key.is_none()
        && patch.deepl_api_key.is_none()
        && patch.baidu_secret_key.is_none()
        && patch.youdao_app_secret.is_none()
}

fn encode(secrets: &Secrets) -> Result<Vec<u8>, String> {
    let json = serde_json::to_vec(secrets).map_err(|err| format!("序列化密钥失败: {err}"))?;
    let mut out = MAGIC.to_vec();
    out.extend_from_slice(hex_encode(&protect(&json)?).as_bytes());
    Ok(out)
}

fn decode(bytes: &[u8]) -> Result<Secrets, String> {
    let rest = bytes
        .strip_prefix(MAGIC)
        .ok_or_else(|| "密钥文件格式无法识别".to_string())?;
    let text = std::str::from_utf8(rest).map_err(|_| "密钥文件编码无效".to_string())?;
    let blob = hex_decode(text.trim())?;
    let json = unprotect(&blob)?;
    serde_json::from_slice(&json).map_err(|err| format!("密钥内容损坏: {err}"))
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn hex_decode(text: &str) -> Result<Vec<u8>, String> {
    if text.len() % 2 != 0 {
        return Err("[DATA] 密钥文件长度无效".into());
    }
    let mut out = Vec::with_capacity(text.len() / 2);
    let bytes = text.as_bytes();
    for index in (0..bytes.len()).step_by(2) {
        let pair = std::str::from_utf8(&bytes[index..index + 2])
            .map_err(|_| "密钥文件编码无效".to_string())?;
        out.push(u8::from_str_radix(pair, 16).map_err(|_| "密钥文件编码无效".to_string())?);
    }
    Ok(out)
}

#[cfg(windows)]
fn protect(plain: &[u8]) -> Result<Vec<u8>, String> {
    dpapi::crypt(plain, true)
}

#[cfg(windows)]
fn unprotect(blob: &[u8]) -> Result<Vec<u8>, String> {
    dpapi::crypt(blob, false)
}

#[cfg(not(windows))]
fn protect(plain: &[u8]) -> Result<Vec<u8>, String> {
    Ok(plain.to_vec())
}

#[cfg(not(windows))]
fn unprotect(blob: &[u8]) -> Result<Vec<u8>, String> {
    Ok(blob.to_vec())
}

#[cfg(windows)]
mod dpapi {
    use std::ffi::c_void;

    #[repr(C)]
    struct DataBlob {
        cb_data: u32,
        pb_data: *mut u8,
    }

    #[link(name = "crypt32")]
    extern "system" {
        fn CryptProtectData(
            p_data_in: *const DataBlob,
            sz_data_descr: *const u16,
            p_optional_entropy: *const DataBlob,
            pv_reserved: *const c_void,
            p_prompt_struct: *const c_void,
            dw_flags: u32,
            p_data_out: *mut DataBlob,
        ) -> i32;
        fn CryptUnprotectData(
            p_data_in: *const DataBlob,
            ppsz_data_descr: *mut *mut u16,
            p_optional_entropy: *const DataBlob,
            pv_reserved: *const c_void,
            p_prompt_struct: *const c_void,
            dw_flags: u32,
            p_data_out: *mut DataBlob,
        ) -> i32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn LocalFree(h_mem: *mut c_void) -> *mut c_void;
    }

    const CRYPTPROTECT_UI_FORBIDDEN: u32 = 0x1;

    pub fn crypt(input: &[u8], encrypt: bool) -> Result<Vec<u8>, String> {
        if input.is_empty() {
            return Ok(Vec::new());
        }
        let data_in = DataBlob {
            cb_data: input.len() as u32,
            pb_data: input.as_ptr() as *mut u8,
        };
        let mut data_out = DataBlob {
            cb_data: 0,
            pb_data: std::ptr::null_mut(),
        };

        let ok = unsafe {
            if encrypt {
                CryptProtectData(
                    &data_in,
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    CRYPTPROTECT_UI_FORBIDDEN,
                    &mut data_out,
                )
            } else {
                CryptUnprotectData(
                    &data_in,
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    CRYPTPROTECT_UI_FORBIDDEN,
                    &mut data_out,
                )
            }
        };
        if ok == 0 {
            return Err(if encrypt {
                "系统加密失败".into()
            } else {
                "系统解密失败".into()
            });
        }

        let bytes = unsafe {
            std::slice::from_raw_parts(data_out.pb_data, data_out.cb_data as usize).to_vec()
        };
        unsafe {
            LocalFree(data_out.pb_data as *mut c_void);
        }
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
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
}
