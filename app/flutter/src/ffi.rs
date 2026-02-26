//! C-ABI FFI functions for embedding the walrus-gateway in Flutter apps.
//!
//! 5 exported functions: `walrus_start`, `walrus_stop`, `walrus_port`,
//! `walrus_last_error`, `walrus_free_string`.

use crate::server;
use std::cell::RefCell;
use std::ffi::{CStr, CString, c_char, c_int};
use std::path::Path;

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

/// Record an error message in the thread-local slot.
fn set_last_error(msg: &str) {
    tracing::error!("{msg}");
    LAST_ERROR.with(|e| {
        *e.borrow_mut() = CString::new(msg).ok();
    });
}

/// Start the embedded gateway. Returns the bound port (> 0) on success,
/// or a negative error code on failure. Call `walrus_last_error` for details.
///
/// # Safety
///
/// `config_dir` must be a valid null-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn walrus_start(config_dir: *const c_char) -> c_int {
    let config_dir = if config_dir.is_null() {
        set_last_error("config_dir is null");
        return -1;
    } else {
        // SAFETY: caller guarantees valid null-terminated string.
        unsafe { CStr::from_ptr(config_dir) }
    };

    let config_dir = match config_dir.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_last_error(&format!("config_dir is not valid UTF-8: {e}"));
            return -1;
        }
    };

    match server::start(Path::new(config_dir)) {
        Ok(port) => port as c_int,
        Err(e) => {
            set_last_error(&format!("failed to start gateway: {e}"));
            -1
        }
    }
}

/// Stop the embedded gateway. Returns 0 on success, -1 on error.
#[unsafe(no_mangle)]
pub extern "C" fn walrus_stop() -> c_int {
    match server::stop() {
        Ok(()) => 0,
        Err(e) => {
            set_last_error(&format!("failed to stop gateway: {e}"));
            -1
        }
    }
}

/// Query the current gateway port. Returns 0 if not running.
#[unsafe(no_mangle)]
pub extern "C" fn walrus_port() -> c_int {
    server::port() as c_int
}

/// Get the last error message. Returns null if no error.
///
/// The returned pointer is valid until the next FFI call on the same thread.
/// Do **not** free it — it is owned by the thread-local storage.
#[unsafe(no_mangle)]
pub extern "C" fn walrus_last_error() -> *const c_char {
    LAST_ERROR.with(|e| {
        e.borrow()
            .as_ref()
            .map(|s| s.as_ptr())
            .unwrap_or(std::ptr::null())
    })
}

/// Free a string previously returned by a walrus FFI function.
///
/// # Safety
///
/// `ptr` must have been allocated by a walrus FFI function, or be null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn walrus_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        // SAFETY: caller guarantees ptr was allocated by CString::into_raw.
        drop(unsafe { CString::from_raw(ptr) });
    }
}
