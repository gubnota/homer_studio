use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopInfo {
    platform: &'static str,
    architecture: &'static str,
    runtime: &'static str,
}

#[tauri::command]
fn desktop_info() -> DesktopInfo {
    DesktopInfo {
        platform: std::env::consts::OS,
        architecture: std::env::consts::ARCH,
        runtime: "Tauri 2",
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![desktop_info])
        .run(tauri::generate_context!())
        .expect("failed to run Homer Studio");
}
