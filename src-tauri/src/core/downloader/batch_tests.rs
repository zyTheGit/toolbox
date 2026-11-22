//! 下载批次（全部完成提醒）的计数与收口逻辑测试。

use crate::models::Job;

use super::state::{batch_summary, begin_batch_if_idle};
use super::Inner;

fn job(status: &str) -> Job {
    Job {
        status: status.into(),
        ..Job::default()
    }
}

#[test]
fn begin_batch_resets_stale_counters_when_idle() {
    let mut inner = Inner {
        batch_open: true,
        batch_success: 3,
        batch_failed: 1,
        ..Inner::default()
    };
    inner.jobs = vec![job("completed"), job("canceled")];

    begin_batch_if_idle(&mut inner);

    assert!(inner.batch_open);
    assert_eq!((inner.batch_success, inner.batch_failed), (0, 0));
}

#[test]
fn begin_batch_keeps_current_batch_while_active() {
    let mut inner = Inner {
        batch_open: true,
        batch_success: 2,
        batch_failed: 0,
        ..Inner::default()
    };
    inner.jobs = vec![job("running")];

    begin_batch_if_idle(&mut inner);

    assert_eq!(inner.batch_success, 2);
}

#[test]
fn summary_waits_until_all_jobs_settle() {
    let mut inner = Inner {
        batch_open: true,
        batch_success: 1,
        batch_failed: 1,
        ..Inner::default()
    };
    inner.jobs = vec![job("completed"), job("queued")];

    assert_eq!(batch_summary(&mut inner), None);
    assert!(inner.batch_open);
    assert_eq!((inner.batch_success, inner.batch_failed), (1, 1));
}

#[test]
fn summary_reports_counts_once_and_resets() {
    let mut inner = Inner {
        batch_open: true,
        batch_success: 3,
        batch_failed: 1,
        ..Inner::default()
    };
    inner.jobs = vec![job("completed"), job("failed")];

    assert_eq!(batch_summary(&mut inner), Some((3, 1)));
    assert!(!inner.batch_open);
    assert_eq!((inner.batch_success, inner.batch_failed), (0, 0));
    assert_eq!(batch_summary(&mut inner), None);
}

#[test]
fn summary_skips_batch_without_results() {
    let mut inner = Inner {
        batch_open: true,
        batch_success: 0,
        batch_failed: 0,
        ..Inner::default()
    };
    inner.jobs = vec![job("canceled")];

    assert_eq!(batch_summary(&mut inner), None);
    assert!(!inner.batch_open);
}
