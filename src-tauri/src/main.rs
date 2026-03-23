#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // 这里的 "clipflow_lib" 对应你 Cargo.toml 里 [lib] 下定义的 name
    // 它负责启动我们在 lib.rs 里写好的 run() 函数
    clipflow_lib::run();
}