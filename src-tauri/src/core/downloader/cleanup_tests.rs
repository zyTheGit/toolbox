use std::path::{Path, PathBuf};

use super::cleanup::{cleanup_temps, temp_stem};
use super::*;

fn work_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("toolbox-cleanup-{tag}-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn touch(dir: &Path, name: &str) {
    std::fs::write(dir.join(name), b"x").unwrap();
}

#[test]
fn cleanup_removes_only_own_temp_artifacts() {
    let dir = work_dir("own");
    for name in [
        "video.mp4",
        "video.mp4.part",
        "video.mp4.part-Frag1.part",
        "video.mp4.part-Frag2",
        "video.mp4.ytdl",
        "video.f137.mp4.part",
        "video2.mp4.part",
        "other.mp4.part",
    ] {
        touch(&dir, name);
    }
    let job = Job {
        out_dir: dir.to_string_lossy().to_string(),
        file: dir.join("video.mp4").to_string_lossy().to_string(),
        ..Job::default()
    };

    assert_eq!(cleanup_temps(&job), 5);
    let left: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect();
    assert!(left.contains(&"video.mp4".to_string()), "成品必须保留: {left:?}");
    assert!(left.contains(&"video2.mp4.part".to_string()));
    assert!(left.contains(&"other.mp4.part".to_string()));
    assert_eq!(left.len(), 3);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn temp_stem_prefers_output_file_then_name_then_log() {
    let mut job = Job::default();
    assert!(temp_stem(&job).is_none());

    job.log = vec!["[download] Destination: C:\\out\\片名 [id].mp4".into()];
    assert_eq!(temp_stem(&job).as_deref(), Some("片名 [id]"));

    job.name = "自定义".into();
    assert_eq!(temp_stem(&job).as_deref(), Some("自定义"));

    job.file = "C:\\out\\自定义.mp4".into();
    assert_eq!(temp_stem(&job).as_deref(), Some("自定义"));
}

#[test]
fn cleanup_ignores_jobs_without_known_prefix() {
    let dir = work_dir("unknown");
    touch(&dir, "anything.mp4.part");
    let job = Job {
        out_dir: dir.to_string_lossy().to_string(),
        ..Job::default()
    };
    assert_eq!(cleanup_temps(&job), 0);
    assert!(dir.join("anything.mp4.part").exists());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn cleanup_tolerates_missing_directory() {
    let job = Job {
        out_dir: std::env::temp_dir()
            .join(format!("toolbox-missing-{}", uuid::Uuid::new_v4()))
            .to_string_lossy()
            .to_string(),
        name: "视频".into(),
        ..Job::default()
    };
    assert_eq!(cleanup_temps(&job), 0);
}
