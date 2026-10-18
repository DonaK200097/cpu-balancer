fn main() {
    // requireAdministrator: exe сам запрашивает UAC (нужно для записи схем, EcoQoS чужих процессов, WMI-событий)
    let windows = tauri_build::WindowsAttributes::new().app_manifest(include_str!("app.manifest"));
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows)).expect("failed to run tauri-build");
}
