use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use encoding_rs::GBK;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::sync::mpsc;

use crate::core::config::home_dir;
use crate::models::TaskPayload;

pub const MAX_LINES: usize = 200;

#[derive(Clone, Debug)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
    pub work_dir: PathBuf,
    pub env: Vec<(String, String)>,
    pub timeout: Duration,
}

#[derive(Clone, Debug)]
pub struct ExecOutcome {
    pub status: &'static str,
    pub exit_code: Option<i32>,
    pub lines: Vec<String>,
    pub error: String,
    pub job_id: String,
}

impl ExecOutcome {
    pub fn failed(error: impl Into<String>) -> Self {
        Self {
            status: "failed",
            exit_code: None,
            lines: Vec::new(),
            error: error.into(),
            job_id: String::new(),
        }
    }

    pub fn canceled() -> Self {
        Self {
            status: "canceled",
            exit_code: None,
            lines: Vec::new(),
            error: "已取消".into(),
            job_id: String::new(),
        }
    }
}

pub fn build_spec(payload: &TaskPayload) -> Result<CommandSpec, String> {
    let command = payload.command.trim();
    if command.is_empty() {
        return Err("[PARAM] 命令内容不能为空".into());
    }
    let env = parse_env(&payload.env)?;
    let work_dir = resolve_work_dir(&payload.work_dir)?;
    let (program, args) = shell_invocation(command, &payload.shell);
    let timeout_seconds = if payload.timeout_seconds == 0 {
        600
    } else {
        payload.timeout_seconds
    };
    Ok(CommandSpec {
        program,
        args,
        work_dir,
        env,
        timeout: Duration::from_secs(timeout_seconds),
    })
}

#[cfg(windows)]
fn shell_invocation(command: &str, shell: &str) -> (String, Vec<String>) {
    if shell.eq_ignore_ascii_case("powershell") {
        (
            "powershell".into(),
            vec!["-NoProfile".into(), "-Command".into(), command.into()],
        )
    } else {
        ("cmd".into(), vec!["/c".into(), command.into()])
    }
}

#[cfg(not(windows))]
fn shell_invocation(command: &str, _shell: &str) -> (String, Vec<String>) {
    ("sh".into(), vec!["-c".into(), command.into()])
}

pub fn resolve_work_dir(raw: &str) -> Result<PathBuf, String> {
    let trimmed = raw.trim();
    let expanded = if trimmed.is_empty() {
        home_dir()
            .map(|path| path.to_string_lossy().to_string())
            .ok_or_else(|| "[PATH] 无法确定用户主目录，请在任务中填写工作目录".to_string())?
    } else {
        expand_path(trimmed)
    };
    if expanded.trim().is_empty() {
        return Err("[PARAM] 工作目录不能为空".into());
    }
    let path = PathBuf::from(&expanded);
    if !path.is_dir() {
        return Err(format!("[PATH] 工作目录不存在: {expanded}"));
    }
    Ok(path)
}

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

pub fn kill_tree(pid: u32) {
    if pid == 0 {
        return;
    }
    #[cfg(windows)]
    {
        use std::process::{Command, Stdio};
        let pid_text = pid.to_string();
        let _ = Command::new("taskkill")
            .args(["/PID", pid_text.as_str(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(unix)]
    {
        use std::process::{Command, Stdio};
        let group = format!("-{pid}");
        let _ = Command::new("kill")
            .args(["-9", group.as_str()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = pid;
    }
}

pub async fn run_command<F, S>(
    spec: CommandSpec,
    cancel: Arc<AtomicBool>,
    on_spawn: S,
    mut on_output: F,
) -> ExecOutcome
where
    F: FnMut(&[String]),
    S: FnOnce(u32),
{
    let mut cmd = tokio::process::Command::new(&spec.program);
    cmd.args(&spec.args)
        .current_dir(&spec.work_dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    for (key, value) in &spec.env {
        cmd.env(key, value);
    }
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000);
    #[cfg(unix)]
    cmd.process_group(0);

    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(err) => return ExecOutcome::failed(format!("[TOOL] 启动命令失败: {err}")),
    };
    let pid = child.id().unwrap_or(0);
    if pid != 0 {
        on_spawn(pid);
    }

    let mut lines: VecDeque<String> = VecDeque::new();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    if let Some(stdout) = child.stdout.take() {
        tokio::spawn(pump(stdout, tx.clone()));
    }
    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(pump(stderr, tx.clone()));
    }
    drop(tx);

    let timeout = if spec.timeout.is_zero() {
        Duration::from_secs(600)
    } else {
        spec.timeout
    };
    let mut wait = Box::pin(child.wait());
    let mut deadline = Box::pin(tokio::time::sleep(timeout));
    let mut ticker = tokio::time::interval(Duration::from_millis(300));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    ticker.tick().await;

    let mut wait_result: Option<std::io::Result<std::process::ExitStatus>> = None;
    let mut timed_out = false;
    let mut killed = false;
    let mut read_closed = false;
    let mut dirty = false;

    loop {
        if wait_result.is_some() {
            break;
        }
        tokio::select! {
            maybe = rx.recv(), if !read_closed => {
                match maybe {
                    Some(line) => {
                        push_line(&mut lines, line);
                        dirty = true;
                    }
                    None => read_closed = true,
                }
            }
            result = &mut wait, if wait_result.is_none() => {
                wait_result = Some(result);
            }
            _ = &mut deadline, if !timed_out && wait_result.is_none() => {
                timed_out = true;
                killed = true;
                kill_tree(pid);
            }
            _ = ticker.tick() => {
                if !killed && wait_result.is_none() && cancel.load(Ordering::SeqCst) {
                    killed = true;
                    kill_tree(pid);
                }
                if dirty {
                    let snapshot: Vec<String> = lines.iter().cloned().collect();
                    on_output(&snapshot);
                    dirty = false;
                }
            }
        }
    }

    let drain_end = tokio::time::Instant::now() + Duration::from_millis(1500);
    loop {
        let remaining = drain_end.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, rx.recv()).await {
            Ok(Some(line)) => push_line(&mut lines, line),
            _ => break,
        }
    }

    let output: Vec<String> = lines.iter().cloned().collect();

    if cancel.load(Ordering::SeqCst) {
        return ExecOutcome {
            status: "canceled",
            exit_code: None,
            lines: output,
            error: "已取消".into(),
            job_id: String::new(),
        };
    }
    if timed_out {
        return ExecOutcome {
            status: "failed",
            exit_code: None,
            lines: output,
            error: "[TIMEOUT] 执行超时，已终止".into(),
            job_id: String::new(),
        };
    }

    match wait_result {
        Some(Ok(status)) => match status.code() {
            Some(0) => ExecOutcome {
                status: "success",
                exit_code: Some(0),
                lines: output,
                error: String::new(),
                job_id: String::new(),
            },
            Some(code) => {
                let tail = last_line(&lines);
                let error = if tail.is_empty() {
                    format!("命令退出码 {code}")
                } else {
                    format!("命令退出码 {code}: {tail}")
                };
                ExecOutcome {
                    status: "failed",
                    exit_code: Some(code),
                    lines: output,
                    error,
                    job_id: String::new(),
                }
            }
            None => ExecOutcome {
                status: "failed",
                exit_code: None,
                lines: output,
                error: "命令被终止".into(),
                job_id: String::new(),
            },
        },
        Some(Err(err)) => ExecOutcome {
            status: "failed",
            exit_code: None,
            lines: output,
            error: format!("读取命令状态失败: {err}"),
            job_id: String::new(),
        },
        None => ExecOutcome {
            status: "failed",
            exit_code: None,
            lines: output,
            error: "命令未正常结束".into(),
            job_id: String::new(),
        },
    }
}

fn push_line(lines: &mut VecDeque<String>, line: String) {
    lines.push_back(line);
    while lines.len() > MAX_LINES {
        lines.pop_front();
    }
}

fn last_line(lines: &VecDeque<String>) -> String {
    lines
        .iter()
        .rev()
        .find(|line| !line.trim().is_empty())
        .map(|line| {
            let trimmed = line.trim();
            if trimmed.chars().count() > 500 {
                let short: String = trimmed.chars().take(500).collect();
                format!("{short}…")
            } else {
                trimmed.to_string()
            }
        })
        .unwrap_or_default()
}

async fn pump<R>(reader: R, sender: mpsc::UnboundedSender<String>)
where
    R: AsyncRead + Unpin + Send + 'static,
{
    let mut reader = BufReader::new(reader);
    let mut buffer: Vec<u8> = Vec::new();
    loop {
        buffer.clear();
        match reader.read_until(b'\n', &mut buffer).await {
            Ok(0) => break,
            Ok(_) => {
                let mut end = buffer.len();
                while end > 0 && (buffer[end - 1] == b'\n' || buffer[end - 1] == b'\r') {
                    end -= 1;
                }
                let bytes = &buffer[..end];
                let line = match std::str::from_utf8(bytes) {
                    Ok(text) => text.to_string(),
                    Err(_) => GBK.decode(bytes).0.into_owned(),
                };
                if sender.send(line).is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    }
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

    #[test]
    fn missing_work_dir_is_rejected() {
        let payload = TaskPayload {
            command: "echo hi".into(),
            work_dir: "definitely-not-a-real-dir-12345".into(),
            ..TaskPayload::default()
        };
        assert!(build_spec(&payload).is_err());
    }

    #[test]
    fn empty_command_is_rejected() {
        let payload = TaskPayload {
            command: "   ".into(),
            ..TaskPayload::default()
        };
        assert!(build_spec(&payload).is_err());
    }

    #[tokio::test]
    async fn timeout_stops_the_process_and_reports_timeout() {
        let (program, args) = if cfg!(windows) {
            ("cmd".to_string(), vec!["/c".into(), "ping -n 10 127.0.0.1 > nul".into()])
        } else {
            ("sh".to_string(), vec!["-c".into(), "sleep 5".into()])
        };
        let spec = CommandSpec {
            program,
            args,
            work_dir: std::env::temp_dir(),
            env: Vec::new(),
            timeout: Duration::from_millis(300),
        };
        let outcome = run_command(spec, Arc::new(AtomicBool::new(false)), |_| {}, |_| {}).await;
        assert_eq!(outcome.status, "failed");
        assert!(outcome.error.starts_with("[TIMEOUT]"));
    }
}
