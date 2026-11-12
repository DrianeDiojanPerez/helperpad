use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

pub fn path() -> PathBuf {
    let state = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .unwrap_or_else(std::env::temp_dir);

    state.join("helperpad/helperpad.log")
}

/// Stdout belongs to Chrome's message stream, so diagnostics go to a file.
pub fn write(line: &str) {
    let path = path();
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{} [{}] {line}", local_time(), std::process::id());
    }
}

fn local_time() -> String {
    let mut buffer = [0u8; 32];
    let written = unsafe {
        let now = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        if libc::localtime_r(&now, &mut tm).is_null() {
            return String::new();
        }
        libc::strftime(buffer.as_mut_ptr().cast(), buffer.len(), c"%Y-%m-%d %H:%M:%S".as_ptr(), &tm)
    };
    String::from_utf8_lossy(&buffer[..written]).into_owned()
}
