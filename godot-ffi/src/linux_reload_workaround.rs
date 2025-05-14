/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Linux-specific configuration.

// Avoid TLS-destructors preventing the dynamic library from being closed.
//
// Credits to fasterthanlime for discovering the very helpful workaround.
// See: https://fasterthanli.me/articles/so-you-want-to-live-reload-rust#what-can-prevent-dlclose-from-unloading-a-library

use std::ffi::c_void;
use std::sync::OnceLock;

static SYSTEM_THREAD_ATEXIT: OnceLock<Option<ThreadAtexitFn>> = OnceLock::new();
static HOT_RELOADING_ENABLED: OnceLock<bool> = OnceLock::new();

pub fn default_set_hot_reload() {
    // By default, we enable hot reloading for debug builds, as it's likely that the user may want hot reloading in debug builds.
    // Release builds however should avoid leaking memory, so we disable hot reloading support by default.
    // In the future, this might consider the .gdextension `is_reloadable` flag, or whether Godot is using an editor or export build.
    if cfg!(debug_assertions) {
        enable_hot_reload()
    } else {
        disable_hot_reload()
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Publicly accessible

#[macro_export]
macro_rules! register_hot_reload_workaround {
    () => {
        #[no_mangle]
        #[doc(hidden)]
        pub unsafe extern "C" fn __cxa_thread_atexit_impl(
            func: *mut ::std::ffi::c_void,
            obj: *mut ::std::ffi::c_void,
            dso_symbol: *mut ::std::ffi::c_void,
        ) {
            $crate::linux_reload_workaround::thread_atexit(func, obj, dso_symbol);
        }
    };
}

type ThreadAtexitFn = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void);

pub fn system_thread_atexit() -> &'static Option<ThreadAtexitFn> {
    SYSTEM_THREAD_ATEXIT.get_or_init(|| unsafe {
        let name = c"__cxa_thread_atexit_impl".as_ptr();
        std::mem::transmute(libc::dlsym(libc::RTLD_NEXT, name))
    })
}

pub fn is_hot_reload_enabled() -> bool {
    // Assume hot reloading is disabled unless something else has been specified already. This is the better default as thread local storage
    // destructors exist for good reasons.
    // This is needed for situations like unit-tests, where we may create TLS-destructors without explicitly calling any of the methods
    // that set hot reloading to be enabled or disabled.
    *HOT_RELOADING_ENABLED.get_or_init(|| false)
}

/// Turns glibc's TLS destructor register function, `__cxa_thread_atexit_impl`,
/// into a no-op if hot reloading is enabled.
///
/// # Safety
/// This needs to be public for symbol visibility reasons, but you should
/// never need to call this yourself
pub unsafe fn thread_atexit(func: *mut c_void, obj: *mut c_void, dso_symbol: *mut c_void) {
    if is_hot_reload_enabled() {
        // Avoid registering TLS destructors on purpose, to avoid
        // double-frees and general crashiness
    } else if let Some(system_thread_atexit) = *system_thread_atexit() {
        // Hot reloading is disabled, and system provides `__cxa_thread_atexit_impl`,
        // so forward the call to it.
        // SAFETY: Is only called by the system when thread_atexit should be called.
        unsafe { system_thread_atexit(func, obj, dso_symbol) };
    } else {
        // Hot reloading is disabled *and* we don't have `__cxa_thread_atexit_impl`,
        // throw hands up in the air and leak memory.
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

fn enable_hot_reload() {
    // If hot reloading is enabled then we should properly unload the library, so this will only be called once.
    HOT_RELOADING_ENABLED
        .set(true)
        .expect("hot reloading should only be set once")
}

fn disable_hot_reload() {
    // If hot reloading is disabled then we may call this method multiple times.
    _ = HOT_RELOADING_ENABLED.set(false)
}
