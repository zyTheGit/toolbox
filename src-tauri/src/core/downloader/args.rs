use super::parse::split_args;

/// yt-dlp options that can start another process or load extra code. They are
/// only allowed when the command-execution switch is on.
const DANGEROUS_ARGS: [&str; 9] = [
    "--exec",
    "--exec-before-download",
    "--config-location",
    "--plugin-dirs",
    "--load-plugins",
    "--downloader",
    "--external-downloader",
    "--update",
    "--update-to",
];

const SECRET_VALUE_ARGS: [&str; 3] = ["--password", "--username", "--video-password"];

pub(super) fn validate_extra_args(extra: &str, allow_commands: bool) -> Result<(), String> {
    if extra.trim().is_empty() || allow_commands {
        return Ok(());
    }
    for token in split_args(extra) {
        let flag = token.split('=').next().unwrap_or(token.as_str()).trim();
        if DANGEROUS_ARGS
            .iter()
            .any(|item| item.eq_ignore_ascii_case(flag))
        {
            return Err(format!(
                "[PARAM] 附加参数包含需要开启命令执行的安全选项：{flag}"
            ));
        }
    }
    Ok(())
}

pub(super) fn render_redacted(args: &[String]) -> String {
    let mut out: Vec<String> = Vec::with_capacity(args.len());
    let mut mask_next = false;
    let mut header_next = false;

    for arg in args {
        let (flag, inline) = split_flag(arg);
        if mask_next {
            out.push("***".into());
            mask_next = false;
            continue;
        }
        if header_next {
            out.push(redact_header(arg));
            header_next = false;
            continue;
        }
        if is_header_flag(flag) {
            match inline {
                Some(value) => out.push(format!("{flag}={}", redact_header(value))),
                None => {
                    out.push(arg.clone());
                    header_next = true;
                }
            }
            continue;
        }
        if SECRET_VALUE_ARGS
            .iter()
            .any(|item| item.eq_ignore_ascii_case(flag))
        {
            match inline {
                Some(_) => out.push(format!("{flag}=***")),
                None => {
                    out.push(arg.clone());
                    mask_next = true;
                }
            }
            continue;
        }
        out.push(redact_text(arg));
    }

    out.iter()
        .map(|arg| {
            if arg.contains(' ') {
                format!("\"{arg}\"")
            } else {
                arg.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn split_flag(arg: &str) -> (&str, Option<&str>) {
    match arg.split_once('=') {
        Some((flag, value)) if flag.starts_with('-') => (flag, Some(value)),
        _ => (arg, None),
    }
}

fn is_header_flag(flag: &str) -> bool {
    flag.eq_ignore_ascii_case("--add-header") || flag.eq_ignore_ascii_case("--header")
}

fn redact_header(value: &str) -> String {
    let name = value.split(':').next().unwrap_or_default().trim().to_lowercase();
    if name == "authorization"
        || name == "cookie"
        || name == "set-cookie"
        || name == "x-api-key"
        || name.contains("token")
        || name.contains("secret")
    {
        "Authorization: ***".into()
    } else {
        redact_text(value)
    }
}

/// Mask credentials in URLs (query values and userinfo) and keep the rest of
/// the line readable. Whitespace is normalized to single spaces.
pub(crate) fn redact_text(input: &str) -> String {
    if !input.contains("://") {
        return input.to_string();
    }
    input
        .split_whitespace()
        .map(redact_token)
        .collect::<Vec<_>>()
        .join(" ")
}

fn redact_token(token: &str) -> String {
    let trimmed = token.trim_matches(|ch| matches!(ch, '"' | '\'' | '(' | ')' | '[' | ']'));
    if !trimmed.contains("://") {
        return token.to_string();
    }
    let Ok(mut url) = reqwest::Url::parse(trimmed) else {
        return token.to_string();
    };
    if url.password().is_some() {
        let _ = url.set_password(Some("***"));
    }
    let pairs: Vec<(String, String)> = url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    if pairs.iter().any(|(key, _)| is_sensitive_key(key)) {
        url.query_pairs_mut().clear().extend_pairs(
            pairs
                .iter()
                .map(|(key, value)| (key.as_str(), if is_sensitive_key(key) { "***" } else { value.as_str() })),
        );
    }
    if token.starts_with('"') && token.ends_with('"') {
        format!("\"{url}\"")
    } else {
        url.to_string()
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    [
        "token",
        "key",
        "sig",
        "sign",
        "auth",
        "secret",
        "password",
        "passwd",
        "session",
        "cookie",
        "expire",
        "credential",
    ]
    .iter()
    .any(|item| key.contains(item))
}
