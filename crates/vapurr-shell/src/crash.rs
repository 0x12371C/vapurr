//! File log + minidump. WER LocalDumps needs HKLM (no admin here).

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Profile, logs, dumps. Same folder as desk.json / Edge / wallet.
/// A zip user must not write into the machine that compiled the exe.
pub fn profile_dir() -> PathBuf {
    crate::desk::Desk::profile_dir()
}

pub fn log_path() -> PathBuf {
    let dir = profile_dir();
    let _ = fs::create_dir_all(&dir);
    dir.join("vapurr.log")
}

pub fn crash_dir() -> PathBuf {
    let dir = profile_dir().join("crashes");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn log(msg: &str) {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if let Ok(mut f) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path())
    {
        let _ = writeln!(f, "[{ts}] {msg}");
        let _ = f.flush();
    }
}

pub fn install() {
    let _ = fs::create_dir_all(crash_dir());
    std::panic::set_hook(Box::new(|info| {
        let loc = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "unknown".into());
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Box<dyn Any>".into()
        };
        log(&format!("PANIC at {loc}: {payload}"));
        write_minidump(std::ptr::null_mut());
    }));
    unsafe {
        SetUnhandledExceptionFilter(Some(unhandled));
    }
    log("crash hooks installed");
}

pub fn write_minidump(exception: *mut ExceptionPointers) {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let path = crash_dir().join(format!("vapurr-{ts}.dmp"));
    let Ok(file) = File::create(&path) else {
        log("minidump: could not create file");
        return;
    };
    use std::os::windows::io::AsRawHandle;
    let handle = file.as_raw_handle();
    let info = MiniDumpExceptionInformation {
        thread_id: unsafe { GetCurrentThreadId() },
        exception_pointers: exception,
        client_pointers: 0,
    };
    let except_ptr = if exception.is_null() {
        std::ptr::null()
    } else {
        &info as *const MiniDumpExceptionInformation
    };
    // MiniDumpNormal | WithIndirectlyReferencedMemory | WithThreadInfo
    const DUMP_TYPE: u32 = 0x0000_1020;
    let ok = unsafe {
        MiniDumpWriteDump(
            GetCurrentProcess(),
            GetCurrentProcessId(),
            handle,
            DUMP_TYPE,
            except_ptr,
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    if ok == 0 {
        log(&format!("minidump FAILED path={}", path.display()));
    } else {
        log(&format!("minidump {}", path.display()));
    }
    let _ = info.thread_id;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logs_live_in_the_user_profile() {
        let p = log_path();
        let s = p.to_string_lossy();
        assert!(s.ends_with("vapurr.log"), "{s}");
        assert!(
            !s.contains("crates") && !s.contains("vapurr-shell"),
            "shipped log must not be CARGO_MANIFEST_DIR: {s}"
        );
        assert!(
            crash_dir().ends_with("crashes"),
            "{}",
            crash_dir().display()
        );
    }
}

extern "system" fn unhandled(info: *mut ExceptionPointers) -> i32 {
    log("unhandled exception");
    write_minidump(info);
    0
}

#[repr(C)]
pub struct ExceptionPointers {
    pub exception_record: *mut std::ffi::c_void,
    pub context_record: *mut std::ffi::c_void,
}

#[repr(C)]
struct MiniDumpExceptionInformation {
    thread_id: u32,
    exception_pointers: *mut ExceptionPointers,
    client_pointers: i32,
}

#[link(name = "dbghelp")]
extern "system" {
    fn MiniDumpWriteDump(
        process: *mut std::ffi::c_void,
        process_id: u32,
        file: *mut std::ffi::c_void,
        dump_type: u32,
        exception: *const MiniDumpExceptionInformation,
        user: *const std::ffi::c_void,
        callback: *const std::ffi::c_void,
    ) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn GetCurrentProcess() -> *mut std::ffi::c_void;
    fn GetCurrentProcessId() -> u32;
    fn GetCurrentThreadId() -> u32;
    fn SetUnhandledExceptionFilter(
        f: Option<extern "system" fn(*mut ExceptionPointers) -> i32>,
    ) -> usize;
}
