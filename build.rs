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
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .ios_path("ios")
        .build();
}
