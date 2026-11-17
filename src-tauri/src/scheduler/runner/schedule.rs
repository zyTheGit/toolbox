//! 任务输入规整与 cron 时间计算。

use std::str::FromStr;

use chrono::{DateTime, Local, NaiveDateTime};
use croner::Cron;
use uuid::Uuid;

use crate::models::{Task, TaskInput, TaskPayload, TaskRun};

use super::command;

pub(super) const NOW_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const NEXT_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const LEGACY_NEXT_FORMAT: &str = "%Y-%m-%d %H:%M";

pub(super) struct TaskFields {
    pub(super) name: String,
    pub(super) task_type: String,
    pub(super) cron: String,
    pub(super) schedule: String,
    pub(super) run_at: String,
    pub(super) enabled: bool,
    pub(super) payload: TaskPayload,
}

pub(super) fn normalize_input(input: &TaskInput) -> Result<TaskFields, String> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err("[PARAM] 请填写任务名称".into());
    }

    let task_type = input.task_type.trim().to_ascii_lowercase();
    if !matches!(task_type.as_str(), "command" | "reminder") {
        return Err("[PARAM] 任务类型仅支持终端或提醒".into());
    }

    let schedule = match input.schedule.trim() {
        "" | "cron" => "cron".to_string(),
        "once" => "once".to_string(),
        _ => return Err("[PARAM] 触发方式无效".into()),
    };

    let (cron, run_at) = if schedule == "once" {
        let run_at = input.run_at.trim();
        let parsed = parse_run_at(run_at)
            .ok_or_else(|| "[PARAM] 指定时间格式应为 YYYY-MM-DD HH:MM:SS".to_string())?;
        if input.enabled && parsed <= Local::now().naive_local() {
            return Err("[PARAM] 指定时间必须晚于当前时间".into());
        }
        (String::new(), run_at.to_string())
    } else {
        let cron = input.cron.trim().to_string();
        parse_cron(&cron)?;
        (cron, String::new())
    };

    let mut payload = input.payload.clone();
    match task_type.as_str() {
        "command" => {
            if payload.command.trim().is_empty() {
                return Err("[PARAM] 命令任务需要填写命令内容".into());
            }
            payload.command = payload.command.trim().to_string();
            payload.shell = if payload.shell.eq_ignore_ascii_case("powershell") {
                "powershell".into()
            } else {
                "cmd".into()
            };
            payload.work_dir = payload.work_dir.trim().to_string();
            if payload.timeout_seconds == 0 {
                payload.timeout_seconds = 600;
            }
            payload.env = payload.env.replace("\r\n", "\n");
            command::parse_env(&payload.env)?;
        }
        "reminder" => {
            payload.message = payload.message.trim().to_string();
        }
        _ => return Err("[PARAM] 任务类型无效".into()),
    }

    Ok(TaskFields {
        name: name.to_string(),
        task_type,
        cron,
        schedule,
        run_at,
        enabled: input.enabled,
        payload,
    })
}

fn parse_cron(expr: &str) -> Result<Cron, String> {
    let expr = expr.trim();
    if expr.is_empty() {
        return Err("[PARAM] cron 表达式不能为空".into());
    }
    if expr.split_whitespace().count() != 5 {
        return Err("[PARAM] cron 表达式需要 5 段（分 时 日 月 周）".into());
    }
    Cron::from_str(expr).map_err(|err| format!("[PARAM] cron 表达式无效: {err}"))
}

pub(super) fn is_once(task: &Task) -> bool {
    task.schedule == "once"
}

pub(super) fn close_once(task: &mut Task) {
    task.enabled = false;
    task.next_run = None;
}

pub(super) fn advance_after_skip(task: &mut Task) {
    if is_once(task) {
        close_once(task);
    } else {
        refresh_next(task);
    }
}

fn parse_run_at(value: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(value.trim(), NEXT_FORMAT).ok()
}

pub(super) fn compute_next(task: &Task) -> Option<String> {
    if !task.enabled {
        return None;
    }
    if is_once(task) {
        return parse_run_at(&task.run_at).map(|time| time.format(NEXT_FORMAT).to_string());
    }
    compute_cron_next(&task.cron)
}

fn compute_cron_next(expr: &str) -> Option<String> {
    let cron = parse_cron(expr).ok()?;
    let next = cron.find_next_occurrence(&Local::now(), false).ok()?;
    Some(format_next(&next))
}

pub(super) fn refresh_next(task: &mut Task) -> bool {
    let next = compute_next(task);
    if task.next_run == next {
        false
    } else {
        task.next_run = next;
        true
    }
}

pub(super) fn preview_cron(expr: &str, count: usize) -> Result<Vec<String>, String> {
    let cron = parse_cron(expr)?;
    let mut cursor = Local::now();
    let mut occurrences = Vec::with_capacity(count);
    for _ in 0..count {
        let next = cron
            .find_next_occurrence(&cursor, false)
            .map_err(|err| format!("cron 表达式无效: {err}"))?;
        occurrences.push(format_next(&next));
        cursor = next;
    }
    Ok(occurrences)
}

fn format_next(time: &DateTime<Local>) -> String {
    time.format(NEXT_FORMAT).to_string()
}

pub(super) fn parse_next(value: &str) -> Option<NaiveDateTime> {
    let value = value.trim();
    NaiveDateTime::parse_from_str(value, NEXT_FORMAT)
        .or_else(|_| NaiveDateTime::parse_from_str(value, LEGACY_NEXT_FORMAT))
        .ok()
}

pub(super) fn now_string() -> String {
    Local::now().format(NOW_FORMAT).to_string()
}

pub(super) fn new_id() -> String {
    Uuid::new_v4().to_string()
}

pub(super) fn command_of(task: &Task) -> String {
    if task.task_type == "command" {
        task.payload.command.clone()
    } else {
        String::new()
    }
}

pub(super) fn build_run(task: &Task, trigger: &str, started_at: &str, status: &str) -> TaskRun {
    TaskRun {
        id: new_id(),
        task_id: task.id.clone(),
        trigger: trigger.to_string(),
        status: status.to_string(),
        started_at: started_at.to_string(),
        finished_at: if status == "running" {
            String::new()
        } else {
            started_at.to_string()
        },
        command: command_of(task),
        ..TaskRun::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn five_field_cron_computes_future_time() {
        let now = Local::now();
        let cron = parse_cron("0 3 * * *").expect("cron should parse");
        let next = cron
            .find_next_occurrence(&now, false)
            .expect("next occurrence");
        assert!(next > now);
        assert_eq!(next.hour(), 3);
        assert_eq!(next.minute(), 0);
        assert_eq!(next.second(), 0);
    }

    #[test]
    fn cron_requires_exactly_five_fields() {
        assert!(parse_cron("0 0 3 * * *").is_err());
        assert!(parse_cron("0 3 * *").is_err());
        assert!(parse_cron("").is_err());
        assert!(parse_cron("nope nope nope nope nope").is_err());
    }

    #[test]
    fn preview_returns_five_timestamp_strings() {
        let preview = preview_cron("*/5 * * * *", 5).expect("preview");
        assert_eq!(preview.len(), 5);
        for item in &preview {
            assert_eq!(item.len(), 19, "unexpected format: {item}");
            assert!(parse_next(item).is_some());
        }
        let mut sorted = preview.clone();
        sorted.sort();
        assert_eq!(sorted, preview);
    }

    #[test]
    fn next_time_format_round_trips_for_stored_value() {
        let next = compute_cron_next("0 3 * * *").expect("next");
        let parsed = parse_next(&next).expect("stored next should parse");
        assert_eq!(next, parsed.format(NEXT_FORMAT).to_string());
    }

    #[test]
    fn legacy_minute_precision_next_run_still_parses() {
        assert!(parse_next("2026-05-15 17:00").is_some());
        assert!(parse_next("2026-05-15 17:00:30").is_some());
        assert!(parse_next("2026-05-15").is_none());
    }

    #[test]
    fn once_schedule_keeps_exact_second() {
        let run_at = (Local::now() + chrono::Duration::days(180))
            .format(NEXT_FORMAT)
            .to_string();
        let task = Task {
            task_type: "reminder".into(),
            schedule: "once".into(),
            run_at: run_at.clone(),
            enabled: true,
            ..Task::default()
        };
        assert_eq!(compute_next(&task), Some(run_at));
    }

    #[test]
    fn once_schedule_requires_seconds() {
        assert!(parse_run_at("2027-05-15 17:00").is_none());
        assert!(parse_run_at("2027-05-15 17:00:30").is_some());
    }

    #[test]
    fn past_once_time_is_rejected_when_enabled() {
        let input = TaskInput {
            name: "续费服务器".into(),
            task_type: "reminder".into(),
            schedule: "once".into(),
            run_at: "2020-01-01 00:00:00".into(),
            enabled: true,
            ..TaskInput::default()
        };
        assert!(normalize_input(&input).is_err());

        let disabled = TaskInput {
            enabled: false,
            ..input
        };
        assert!(normalize_input(&disabled).is_ok());
    }

    #[test]
    fn unsupported_task_type_is_rejected() {
        let base = TaskInput {
            name: "旧任务".into(),
            enabled: false,
            ..TaskInput::default()
        };
        for task_type in ["download", "translate", "other"] {
            let input = TaskInput {
                task_type: task_type.into(),
                ..base.clone()
            };
            assert!(normalize_input(&input).is_err(), "{task_type} should be rejected");
        }
    }
}
