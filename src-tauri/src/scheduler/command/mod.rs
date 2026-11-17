//! 终端命令任务：规格构建、平台差异与进程控制。

mod exec;
mod vars;

pub use exec::run_command;
pub use vars::{expand_path, expand_vars, parse_env};

use std::path::PathBuf;
use std::time::Duration;

use crate::core::config::home_dir;
use crate::models::TaskPayload;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
