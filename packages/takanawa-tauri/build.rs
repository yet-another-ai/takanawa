const COMMANDS: &[&str] = &[
    "create",
    "start",
    "pause",
    "cancel",
    "snapshot",
    "bitmap",
    "close",
    "download_to_completion",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
