//! 系统对话框入口。测试构建返回空值，避免把 rfd 的 ComCtl32 v6
//! 依赖带进测试可执行文件（测试进程没有应用程序清单）。

use std::path::PathBuf;

#[cfg(not(test))]
pub fn save_file(title: &str, file_name: &str) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title(title)
        .set_file_name(file_name)
        .add_filter("JSON", &["json"])
        .save_file()
}

#[cfg(not(test))]
pub fn pick_file(title: &str) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title(title)
        .add_filter("JSON", &["json"])
        .pick_file()
}

#[cfg(not(test))]
pub fn pick_sqlite_file() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("选择 SQLite 数据库")
        .add_filter("SQLite", &["db", "sqlite", "sqlite3", "db3"])
        .add_filter("所有文件", &["*"])
        .pick_file()
}

#[cfg(not(test))]
pub fn pick_folder() -> Option<PathBuf> {
    rfd::FileDialog::new().pick_folder()
}

#[cfg(not(test))]
pub fn confirm(title: &str, description: &str) -> bool {
    matches!(
        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Warning)
            .set_title(title)
            .set_description(description)
            .set_buttons(rfd::MessageButtons::YesNo)
            .show(),
        rfd::MessageDialogResult::Yes
    )
}

#[cfg(not(test))]
pub fn info_later(title: String, body: &str) {
    let body = body.to_string();
    std::thread::spawn(move || {
        let _ = rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Info)
            .set_title(&title)
            .set_description(&body)
            .show();
    });
}

#[cfg(test)]
#[allow(dead_code)]
pub fn save_file(_title: &str, _file_name: &str) -> Option<PathBuf> {
    None
}

#[cfg(test)]
#[allow(dead_code)]
pub fn pick_file(_title: &str) -> Option<PathBuf> {
    None
}

#[cfg(test)]
#[allow(dead_code)]
pub fn pick_sqlite_file() -> Option<PathBuf> {
    None
}

#[cfg(test)]
#[allow(dead_code)]
pub fn pick_folder() -> Option<PathBuf> {
    None
}

#[cfg(test)]
#[allow(dead_code)]
pub fn confirm(_title: &str, _description: &str) -> bool {
    false
}

#[cfg(test)]
#[allow(dead_code)]
pub fn info_later(_title: String, _body: &str) {}
