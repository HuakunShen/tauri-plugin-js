const COMMANDS: &[&str] = &[
    "spawn",
    "kill",
    "kill_all",
    "restart",
    "list_processes",
    "get_status",
    "write_stdin",
    "detect_runtimes",
    "set_runtime_path",
    "get_runtime_paths",
];

fn main() {
    // Make target triple available at compile time for sidecar resolution
    println!(
        "cargo:rustc-env=TARGET_TRIPLE={}",
        std::env::var("TARGET").unwrap()
    );

    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .ios_path("ios")
        .build();
}
