//! Папка данных: рядом с exe (portable), а если туда писать нельзя — %APPDATA%\CPU Balancer.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn exe_path() -> PathBuf {
    std::env::current_exe().unwrap_or_else(|_| PathBuf::from("cpu-balancer.exe"))
}

fn writable(dir: &Path) -> bool {
    let probe = dir.join(".cpu-balancer-write-test");
    match std::fs::write(&probe, b"") {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

pub fn data_dir() -> &'static Path {
    DATA_DIR.get_or_init(|| {
        if let Ok(dir) = std::env::var("CPU_BALANCER_DATA") {
            return PathBuf::from(dir);
        }
        let exe_dir = exe_path().parent().map(Path::to_path_buf).unwrap_or_default();
        if writable(&exe_dir) {
            return exe_dir;
        }
        let appdata = std::env::var("APPDATA").map(PathBuf::from).unwrap_or(exe_dir);
        let dir = appdata.join("CPU Balancer");
        let _ = std::fs::create_dir_all(&dir);
        dir
    })
}

pub fn file(name: &str) -> PathBuf {
    data_dir().join(name)
}

/// Читает текст, убирая BOM (PowerShell 5.1 пишет UTF-8 с BOM).
pub fn read_text(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF][..]).unwrap_or(&bytes);
    Some(String::from_utf8_lossy(bytes).into_owned())
}

/// Атомарная запись: во временный файл и переименование.
pub fn write_text(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, text.as_bytes())?;
    std::fs::rename(&tmp, path)
}
