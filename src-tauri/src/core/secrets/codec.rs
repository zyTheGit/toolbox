//! 密钥文件编码：magic + hex(DPAPI 保护的 JSON)。

use super::{Secrets, SecretsV1};

const MAGIC_V1: &[u8] = b"TOOLBOX-SECRETS-1\n";
const MAGIC_V2: &[u8] = b"TOOLBOX-SECRETS-2\n";

pub(super) enum Decoded {
    Current(Secrets),
    Legacy(SecretsV1),
}

pub(super) fn encode(secrets: &Secrets) -> Result<Vec<u8>, String> {
    let json = serde_json::to_vec(secrets).map_err(|err| format!("序列化密钥失败: {err}"))?;
    let mut out = MAGIC_V2.to_vec();
    out.extend_from_slice(hex_encode(&protect(&json)?).as_bytes());
    Ok(out)
}

pub(super) fn decode(bytes: &[u8]) -> Result<Decoded, String> {
    if let Some(rest) = bytes.strip_prefix(MAGIC_V2) {
        let json = unprotect_payload(rest)?;
        let secrets: Secrets =
            serde_json::from_slice(&json).map_err(|err| format!("密钥内容损坏: {err}"))?;
        Ok(Decoded::Current(secrets))
    } else if let Some(rest) = bytes.strip_prefix(MAGIC_V1) {
        let json = unprotect_payload(rest)?;
        let legacy: SecretsV1 =
            serde_json::from_slice(&json).map_err(|err| format!("密钥内容损坏: {err}"))?;
        Ok(Decoded::Legacy(legacy))
    } else {
        Err("密钥文件格式无法识别".into())
    }
}

#[cfg(test)]
pub(super) fn encode_legacy(legacy: &SecretsV1) -> Result<Vec<u8>, String> {
    let json = serde_json::to_vec(legacy).map_err(|err| format!("序列化密钥失败: {err}"))?;
    let mut out = MAGIC_V1.to_vec();
    out.extend_from_slice(hex_encode(&protect(&json)?).as_bytes());
    Ok(out)
}

fn unprotect_payload(rest: &[u8]) -> Result<Vec<u8>, String> {
    let text = std::str::from_utf8(rest).map_err(|_| "密钥文件编码无效".to_string())?;
    let blob = hex_decode(text.trim())?;
    unprotect(&blob)
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
