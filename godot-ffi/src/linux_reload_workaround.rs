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
//
// We do not export `__cxa_thread_atexit_impl`: Godot loads extensions with `RTLD_DEEPBIND`, so their dependencies (e.g. libstdc++) calling it
// would run our instead of glibc's fn definition. glibc then keeps the extension loaded as long as libstdc++, which is never unloaded
// (`STB_GNU_UNIQUE`) -> hot reload would encounter stale statics. Not exporting it only affects other libraries; Rust std still links to it.

use std::ffi::c_void;
use std::sync::OnceLock;

static SYSTEM_THREAD_ATEXIT: OnceLock<Option<ThreadAtexitFn>> = OnceLock::new();
static HOT_RELOADING_ENABLED: OnceLock<bool> = OnceLock::new();

pub fn default_set_hot_reload() {
    // By default, we enable hot reloading for debug builds, as it's likely that the user may want hot reloading in debug builds.
    // Release builds however should avoid leaking memory, so we disable hot reloading support by default.
    // In the future, this might consider the .gdextension `is_reloadable` flag, or whether Godot is using an editor or export build.
    let enabled = cfg!(debug_assertions);

    // Already set -> `dlclose` did not unmap the library; continuing would re-initialize stale statics.
    if HOT_RELOADING_ENABLED.set(enabled).is_err() {
        panic!(
            "GDExtension library was re-initialized without being unloaded first. Hot-reload is not possible, restart Godot.\n\
            Something keeps the library loaded after `dlclose`, e.g. thread-local destructors or a library binding to its symbols."
        );
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Publicly accessible

// Hidden symbol needs asm: stable Rust `#[no_mangle]` always exports, visibility control is nightly-only, and a dependency crate cannot pass
// a `--version-script` to the user's cdylib. Other architectures don't define the symbol -> TLS destructors block hot reload.
#[macro_export]
macro_rules! register_hot_reload_workaround {
    () => {
        // Module, as `global_asm!` cannot be in statement position (e.g. doctests with `#[gdextension]` inside `fn main`).
        mod __godot_rust_hot_reload {
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            $crate::__thread_atexit_trampoline!("jmp {f}");
            #[cfg(target_arch = "aarch64")]
            $crate::__thread_atexit_trampoline!("b {f}");
            #[cfg(target_arch = "riscv64")]
            $crate::__thread_atexit_trampoline!("tail {f}");
        }
    };
}

#[macro_export]
macro_rules! __thread_atexit_trampoline {
    ($jump:literal) => {
        ::std::arch::global_asm!(
            ".pushsection .text",
            ".globl __cxa_thread_atexit_impl",
            ".hidden __cxa_thread_atexit_impl",
            ".type __cxa_thread_atexit_impl, %function",
            "__cxa_thread_atexit_impl:",
            $jump,
            ".popsection",
            f = sym $crate::linux_reload_workaround::thread_atexit,
        );
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
    //
    // Do *not* eagerly initialize here. Our `__cxa_thread_atexit_impl` replaces the one from glibc, so any thread can call `thread_atexit`
    // (and thus this function) before `__gdext_load_library` runs. If we stored `false` on that first read, the later `default_set_hot_reload()`
    // would try to set the value a second time and panic. See https://github.com/godot-rust/gdext/issues/1209.
    HOT_RELOADING_ENABLED.get().copied().unwrap_or(false)
}

/// Turns glibc's TLS destructor register function, `__cxa_thread_atexit_impl`,
/// into a no-op if hot reloading is enabled.
///
/// # Safety
/// This needs to be public for symbol visibility reasons, but you should never need to call this yourself.
pub unsafe extern "C" fn thread_atexit(
    func: *mut c_void,
    obj: *mut c_void,
    dso_symbol: *mut c_void,
) {
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

#[cfg(test)]
mod tests {
    use super::*;

    // Single test, as the OnceLock is global: early read must not block init + second init (no unload) panics.
    // See https://github.com/godot-rust/gdext/issues/1209.
    #[test]
    fn hot_reload_setting_lifecycle() {
        // Simulates `thread_atexit` firing before `__gdext_load_library` runs.
        let early = is_hot_reload_enabled();
        assert!(!early, "default must be false before any enable/disable");

        // Mimics what `__gdext_load_library` does next.
        default_set_hot_reload();

        // `default_set_hot_reload()` enables only under `debug_assertions`; release-mode test runs disable instead.
        assert_eq!(is_hot_reload_enabled(), cfg!(debug_assertions));

        // Re-init of a library that was not unloaded.
        let reinit = std::panic::catch_unwind(default_set_hot_reload);
        assert!(
            reinit.is_err(),
            "re-initialization with stale statics must panic"
        );
    }
}
