//! 命令执行：子进程生命周期、输出采集与超时/取消控制。

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use encoding_rs::GBK;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::sync::mpsc;

use super::{kill_tree, CommandSpec, ExecOutcome};

const MAX_LINES: usize = 200;

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
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
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
    // 发出终止信号后的兜底期限：即使回收失败也不再阻塞调用方。
    let mut give_up: Option<tokio::time::Instant> = None;

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
                give_up = Some(tokio::time::Instant::now() + Duration::from_secs(3));
                kill_tree(pid);
            }
            _ = ticker.tick() => {
                if !killed && wait_result.is_none() && cancel.load(Ordering::SeqCst) {
                    killed = true;
                    give_up = Some(tokio::time::Instant::now() + Duration::from_secs(3));
                    kill_tree(pid);
                }
                if give_up.is_some_and(|limit| tokio::time::Instant::now() >= limit) {
                    break;
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
                let shell = shell_label(&spec.program);
                let mut error = if tail.is_empty() {
                    format!("命令退出码 {code}（{shell}）")
                } else {
                    format!("命令退出码 {code}（{shell}）: {tail}")
                };
                // cmd 下误用 PowerShell 语法（如行首 & 调用符）是最常见的失败原因，
                // 附提示引导到任务的 Shell 选项，降低排错成本。
                #[cfg(windows)]
                if spec.program.eq_ignore_ascii_case("cmd") {
                    error.push_str("。提示：cmd 不支持 PowerShell 语法（如 & 调用符），可在任务高级选项中切换 Shell");
                }
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

/// 展示用的 Shell 名称标签。
fn shell_label(program: &str) -> &'static str {
    if program.eq_ignore_ascii_case("powershell") {
        "PowerShell"
    } else if program.eq_ignore_ascii_case("cmd") {
        "cmd"
    } else if program == "sh" {
        "sh"
    } else {
        "未知 Shell"
    }
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
