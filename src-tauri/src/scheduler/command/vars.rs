//! 路径展开与环境变量解析。

use crate::core::config::home_dir;

pub fn expand_path(value: &str) -> String {
    expand_vars(value.trim())
}

pub fn parse_env(raw: &str) -> Result<Vec<(String, String)>, String> {
    let mut pairs = Vec::new();
    for (index, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (key, value) = line.split_once('=').ok_or_else(|| {
            format!("[PARAM] 环境变量格式无效（第 {} 行）: {line}", index + 1)
        })?;
        let key = key.trim();
        let value = value.trim();
        if key.is_empty() {
            return Err(format!("[PARAM] 环境变量名不能为空（第 {} 行）", index + 1));
        }
        if !key
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            return Err(format!("[PARAM] 环境变量名无效（第 {} 行）: {key}", index + 1));
        }
        pairs.push((key.to_string(), value.to_string()));
    }
    Ok(pairs)
}

pub fn expand_vars(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];

        if ch == '~' && i == 0 && (chars.len() == 1 || matches!(chars[1], '/' | '\\')) {
            if let Some(home) = home_dir() {
                out.push_str(&home.to_string_lossy());
                i += 1;
                continue;
            }
        }

        if ch == '%' {
            if let Some(offset) = chars[i + 1..].iter().position(|c| *c == '%') {
                let end = i + 1 + offset;
                let name: String = chars[i + 1..end].iter().collect();
                if is_var_name(&name) {
                    if let Ok(value) = std::env::var(&name) {
                        out.push_str(&value);
                        i = end + 1;
                        continue;
                    }
                }
            }
        }

        if ch == '$' {
            if chars.get(i + 1) == Some(&'{') {
                if let Some(offset) = chars[i + 2..].iter().position(|c| *c == '}') {
                    let end = i + 2 + offset;
                    let name: String = chars[i + 2..end].iter().collect();
                    if is_var_name(&name) {
                        if let Ok(value) = std::env::var(&name) {
                            out.push_str(&value);
                            i = end + 1;
                            continue;
                        }
                    }
                }
            }

            let mut name_start = i + 1;
            let prefix_end = (i + 5).min(chars.len());
            let prefix: String = chars[i + 1..prefix_end].iter().collect();
            if prefix.eq_ignore_ascii_case("env:") {
                name_start = i + 5;
            }
            let (name, end) = take_var_name(&chars, name_start);
            if !name.is_empty() {
                if let Ok(value) = std::env::var(&name) {
                    out.push_str(&value);
                    i = end;
                    continue;
                }
            }
        }

        out.push(ch);
        i += 1;
    }
    out
}

fn is_var_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        && !name.chars().all(|ch| ch.is_ascii_digit())
}

fn take_var_name(chars: &[char], start: usize) -> (String, usize) {
    let mut end = start;
    while end < chars.len() && (chars[end].is_ascii_alphanumeric() || chars[end] == '_') {
        end += 1;
    }
    (chars[start..end].iter().collect(), end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_env_lines() {
        let pairs = parse_env("A=1\r\n\r\nB=hello world\n").unwrap();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0], ("A".to_string(), "1".to_string()));
        assert_eq!(pairs[1], ("B".to_string(), "hello world".to_string()));
    }

    #[test]
    fn rejects_invalid_env_lines() {
        assert!(parse_env("A=1\nBAD-LINE").is_err());
        assert!(parse_env("=1").is_err());
        assert!(parse_env("A B=1").is_err());
    }

    #[test]
    fn expands_env_vars() {
        std::env::set_var("TOOLBOX_SCHED_TEST_VAR", "toolbox-value");
        assert_eq!(expand_vars("%TOOLBOX_SCHED_TEST_VAR%"), "toolbox-value");
        assert_eq!(expand_vars("$TOOLBOX_SCHED_TEST_VAR"), "toolbox-value");
        assert_eq!(expand_vars("${TOOLBOX_SCHED_TEST_VAR}"), "toolbox-value");
        assert_eq!(expand_vars("$env:TOOLBOX_SCHED_TEST_VAR"), "toolbox-value");
        assert_eq!(expand_vars("x%TOOLBOX_SCHED_TEST_VAR%y"), "xtoolbox-valuey");
    }
}
