//! 进程清理：终止子进程及其派生的整棵进程树。
//!
//! 子进程统一用 `process_group(0)` 放进独立进程组，清理时可直接向该组发信号。
//! 不要通过 `kill -9 -<pid>` 子进程间接完成：procps 的 `kill` 对负 PID 参数解析
//! 不一致，Ubuntu 22.04 上会把信号打到调用方自己所在的进程组，导致整个测试进程树
//! 被杀（2026-11 Ubuntu CI 单测 exit 137 的根因）。

/// 结束 `pid` 及其子进程。
pub fn kill_tree(pid: u32) {
    if pid == 0 || pid == std::process::id() {
        return;
    }
    #[cfg(windows)]
    kill_tree_windows(pid);
    #[cfg(unix)]
    kill_tree_unix(pid);
    #[cfg(not(any(windows, unix)))]
    let _ = pid;
}

#[cfg(windows)]
fn kill_tree_windows(pid: u32) {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    const NO_WINDOW: u32 = 0x0800_0000;

    let pid_text = pid.to_string();
    let _ = Command::new("taskkill")
        .args(["/PID", pid_text.as_str(), "/T", "/F"])
        .creation_flags(NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// 仅当目标是进程组组长时按组发送，避免信号落到自身所在的组。
#[cfg(unix)]
fn kill_tree_unix(pid: u32) {
    let pid = pid as libc::pid_t;
    // SAFETY: getpgid/kill 只读取进程属性或发送信号，不涉及内存安全。
    unsafe {
        let group = libc::getpgid(pid);
        let own_group = libc::getpgid(0);
        if group == pid && group != own_group {
            libc::kill(-pid, libc::SIGKILL);
        }
        libc::kill(pid, libc::SIGKILL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn kills_child_in_its_own_process_group() {
        use std::os::unix::process::CommandExt;
        use std::time::{Duration, Instant};

        let mut command = std::process::Command::new("sh");
        command.args(["-c", "sleep 30"]).process_group(0);
        let mut child = command.spawn().expect("启动测试子进程");
        kill_tree(child.id());

        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if child.try_wait().expect("读取子进程状态").is_some() {
                return;
            }
            assert!(Instant::now() < deadline, "kill_tree 未能在 5 秒内结束子进程");
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}
