use std::sync::atomic::{AtomicBool, Ordering};

static QUIT_REQUESTED: AtomicBool = AtomicBool::new(false);

pub fn should_quit() -> bool {
    QUIT_REQUESTED.load(Ordering::SeqCst)
}

pub fn request_quit() {
    QUIT_REQUESTED.store(true, Ordering::SeqCst);
}

#[cfg(unix)]
pub fn install() {
    use std::os::raw::c_int;
    const SIGINT: c_int = 2;
    const SIGTERM: c_int = 15;

    extern "C" fn handler(_sig: c_int) {
        // Set a flag only; do not perform IO in signal context
        request_quit();
    }

    extern "C" {
        fn signal(sig: c_int, handler: extern "C" fn(c_int)) -> usize;
    }

    unsafe {
        // Best-effort; ignore returns
        let _ = signal(SIGINT, handler);
        let _ = signal(SIGTERM, handler);
    }
}

#[cfg(not(unix))]
pub fn install() {
    // Windows console Ctrl+C handler via SetConsoleCtrlHandler
    #[cfg(target_os = "windows")]
    unsafe {
        use std::ptr;
        type HandlerRoutine = extern "system" fn(u32) -> i32;
        extern "system" {
            fn SetConsoleCtrlHandler(handler: Option<HandlerRoutine>, add: i32) -> i32;
        }
        extern "system" fn handler(ctrl_type: u32) -> i32 {
            // Handle CTRL_C_EVENT(0), CTRL_CLOSE_EVENT(2) etc.
            let _ = ctrl_type; // unused detail
            request_quit();
            1 // handled
        }
        let _ = SetConsoleCtrlHandler(Some(handler), 1);
    }
    #[cfg(not(target_os = "windows"))]
    {
        // No-op fallback; periodic autosave still provides coverage
    }
}
