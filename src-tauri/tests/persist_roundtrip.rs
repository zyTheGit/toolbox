use std::path::PathBuf;

use toolbox_lib::core::persist;

fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("toolbox-it-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn corrupted_data_is_never_overwritten() {
    let dir = temp_dir();
    let path = dir.join("history.json");
    std::fs::write(&path, "{ broken").unwrap();

    let loaded = persist::load_json::<Vec<String>>(&path);
    assert!(loaded.is_err());
    assert!(!path.exists(), "corrupt source must be moved aside");

    let backups: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("history.json.corrupt-")
        })
        .collect();
    assert_eq!(backups.len(), 1);
    assert_eq!(
        std::fs::read_to_string(backups[0].path()).unwrap(),
        "{ broken"
    );
}

#[test]
fn concurrent_saves_leave_a_readable_file() {
    let dir = temp_dir();
    let path = dir.join("jobs.json");
    let mut handles = Vec::new();
    for index in 0..16 {
        let path = path.clone();
        handles.push(std::thread::spawn(move || {
            let value = vec![format!("job-{index}"); 48];
            persist::save_json(&path, &value).unwrap();
        }));
    }
    for handle in handles {
        handle.join().unwrap();
    }

    let loaded: Vec<String> = persist::load_json(&path).unwrap().unwrap();
    assert_eq!(loaded.len(), 48);
    let leftovers = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with('.'))
        .count();
    assert_eq!(leftovers, 0, "temporary files must be cleaned up");
}