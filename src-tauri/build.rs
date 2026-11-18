fn main() {
    tauri_build::build();

    // 测试可执行文件没有 Tauri 生成的应用程序清单，直接加载会因
    // ComCtl32 v6 入口缺失报 STATUS_ENTRYPOINT_NOT_FOUND。改为延迟加载：
    // 测试不调用对话框就不会解析入口，正式程序仍由清单激活 v6。
    #[cfg(windows)]
    {
        println!("cargo:rustc-link-arg=/DELAYLOAD:comctl32.dll");
        println!("cargo:rustc-link-arg=delayimp.lib");
    }
}
