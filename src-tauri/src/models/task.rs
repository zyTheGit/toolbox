//! 定时任务模型。

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TaskPayload {
    pub url: String,
    pub quality: String,
    pub text: String,
    pub direction: String,
    pub save_file: String,
    pub command: String,
    pub work_dir: String,
    pub shell: String,
    pub timeout_seconds: u64,
    pub env: String,
    pub message: String,
}

impl Default for TaskPayload {
    fn default() -> Self {
        Self {
            url: String::new(),
            quality: "best".into(),
            text: String::new(),
            direction: "auto".into(),
            save_file: String::new(),
            command: String::new(),
            work_dir: String::new(),
            shell: "cmd".into(),
            timeout_seconds: 600,
            env: String::new(),
            message: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub task_type: String,
    pub cron: String,
    #[serde(default = "default_schedule")]
    pub schedule: String,
    #[serde(default)]
    pub run_at: String,
    pub enabled: bool,
    pub payload: TaskPayload,
    pub created_at: String,
    pub updated_at: String,
    pub last_run_at: String,
    pub last_status: String,
    pub last_output: String,
    pub last_error: String,
    pub last_job_id: String,
    pub running: bool,
    pub next_run: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TaskInput {
    pub name: String,
    #[serde(rename = "type")]
    pub task_type: String,
    pub cron: String,
    #[serde(default = "default_schedule")]
    pub schedule: String,
    #[serde(default)]
    pub run_at: String,
    pub enabled: bool,
    pub payload: TaskPayload,
}

impl Default for TaskInput {
    fn default() -> Self {
        Self {
            name: String::new(),
            task_type: "command".into(),
            cron: "0 3 * * *".into(),
            schedule: default_schedule(),
            run_at: String::new(),
            enabled: true,
            payload: TaskPayload::default(),
        }
    }
}

fn default_schedule() -> String {
    "cron".into()
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct TaskRun {
    pub id: String,
    pub task_id: String,
    pub trigger: String,
    pub status: String,
    pub started_at: String,
    pub finished_at: String,
    pub duration_ms: u64,
    pub exit_code: Option<i32>,
    pub output: String,
    pub error: String,
    pub job_id: String,
    pub command: String,
}
