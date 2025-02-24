/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Distributed self-registration of "plugins" without central list

// Note: code in this file is safe, however it seems that some annotations fall into the "unsafe" category.
// For example, adding #![forbid(unsafe_code)] causes this error:
//   note: the program's behavior with overridden link sections on items is unpredictable
//   and Rust cannot provide guarantees when you manually override them

/// Declare a global registry for plugins with a given name
#[doc(hidden)]
#[macro_export]
macro_rules! plugin_registry {
    ($vis:vis $registry:ident: $Type:ty) => {
        $crate::paste::paste! {
            #[used]
            #[allow(non_upper_case_globals)]
            #[doc(hidden)]
            $vis static [< __godot_rust_plugin_ $registry >]:
                std::sync::Mutex<Vec<$Type>> = std::sync::Mutex::new(Vec::new());
        }
    };
}

#[doc(hidden)]
#[macro_export]
#[allow(clippy::deprecated_cfg_attr)]
#[cfg_attr(rustfmt, rustfmt::skip)]
// ^ skip: paste's [< >] syntax chokes fmt
//   cfg_attr: workaround for https://github.com/rust-lang/rust/pull/52234#issuecomment-976702997
macro_rules! plugin_execute_pre_main_wasm {
    ($gensym:ident,) => {
        // Rust presently requires that statics with a custom `#[link_section]` must be a simple
        // list of bytes on the wasm target (with no extra levels of indirection such as references).
        //
        // As such, instead we export a fn with a random name of predictable format to be used
        // by the embedder.
        $crate::paste::paste! {
            #[no_mangle]
            extern "C" fn [< rust_gdext_registrant_ $gensym >] () {
                __init();
            }
        }
    };
}

/// Executes a block of code before main, by utilising platform specific linker instructions.
#[doc(hidden)]
#[macro_export]
#[allow(clippy::deprecated_cfg_attr)]
#[cfg_attr(rustfmt, rustfmt::skip)]
// ^ skip: paste's [< >] syntax chokes fmt
//   cfg_attr: workaround for https://github.com/rust-lang/rust/pull/52234#issuecomment-976702997
macro_rules! plugin_execute_pre_main {
    ($body:expr) => {
        const _: () = {
            #[allow(non_upper_case_globals)]
            #[used]
            // Windows:
            #[cfg_attr(target_os = "windows", link_section = ".CRT$XCU")]
            // MacOS + iOS:
            #[cfg_attr(target_os = "ios", link_section = "__DATA,__mod_init_func")]
            #[cfg_attr(target_os = "macos", link_section = "__DATA,__mod_init_func")]
            // Linux, Android, BSD:
            #[cfg_attr(target_os = "android", link_section = ".init_array")]
            #[cfg_attr(target_os = "dragonfly", link_section = ".init_array")]
            #[cfg_attr(target_os = "freebsd", link_section = ".init_array")]
            #[cfg_attr(target_os = "linux", link_section = ".init_array")]
            #[cfg_attr(target_os = "netbsd", link_section = ".init_array")]
            #[cfg_attr(target_os = "openbsd", link_section = ".init_array")]
            static __init: extern "C" fn() = {
                #[cfg_attr(target_os = "android", link_section = ".text.startup")]
                #[cfg_attr(target_os = "linux", link_section = ".text.startup")]
                extern "C" fn __inner_init() {
                    $body
                }
                __inner_init
            };

            #[cfg(target_family = "wasm")]
            $crate::gensym! { $crate::plugin_execute_pre_main_wasm!() }
        };
    };
}

/// register a plugin by executing code pre-main that adds the plugin to the plugin registry
#[doc(hidden)]
#[macro_export]
#[allow(clippy::deprecated_cfg_attr)]
#[cfg_attr(rustfmt, rustfmt::skip)]
// ^ skip: paste's [< >] syntax chokes fmt
//   cfg_attr: workaround for https://github.com/rust-lang/rust/pull/52234#issuecomment-976702997
macro_rules! plugin_add_inner {
    ($registry:ident; $plugin:expr; $( $path_tt:tt )* ) => {
        $crate::plugin_execute_pre_main!({
            let mut guard = $crate::paste::paste!( $( $path_tt )* [< __godot_rust_plugin_ $registry >] )
                .lock()
                .unwrap();
            guard.push($plugin);
        });
    };
}

/// Register a plugin to a registry
#[doc(hidden)]
#[macro_export]
macro_rules! plugin_add {
    ( $registry:ident; $plugin:expr ) => {
		$crate::plugin_add_inner!($registry; $plugin; );
	};

    ( $registry:ident in $path:path; $plugin:expr ) => {
		$crate::plugin_add_inner!($registry; $plugin; $path ::);
	};
}

/// Iterate over all plugins in unspecified order
#[doc(hidden)]
#[macro_export]
macro_rules! plugin_foreach_inner {
    ( $registry:ident; $closure:expr; $( $path_tt:tt )* ) => {
        let guard = $crate::paste::paste!( $( $path_tt )* [< __godot_rust_plugin_ $registry >] )
            .lock()
            .unwrap();

        for e in guard.iter() {
            #[allow(clippy::redundant_closure_call)]
            $closure(e);
        }
    };
}

/// Register a plugin to a registry
#[doc(hidden)]
#[macro_export]
macro_rules! plugin_foreach {
    ( $registry:ident; $closure:expr ) => {
		$crate::plugin_foreach_inner!($registry; $closure; );
	};

    ( $registry:ident in $path:path; $closure:expr ) => {
		$crate::plugin_foreach_inner!($registry; $closure; $path ::);
	};
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[cfg(test)] #[cfg_attr(published_docs, doc(cfg(test)))]
mod tests {
    use std::collections::HashSet;
    plugin_registry!(V: &'static str);

    plugin_add!(V; "three");
    plugin_add!(V; "four");
    plugin_add!(V; "one");
    plugin_add!(V; "two");

    #[test]
    fn plugin_registry() {
        let expected = HashSet::from(["one", "two", "three", "four"]);
        let mut actual = HashSet::new();

        plugin_foreach!(V; |e: &'static str| {
            actual.insert(e);
        });

        assert_eq!(actual, expected);
    }
}
