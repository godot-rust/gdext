/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Distributed self-registration of "shards" without central list.

/// Declare a global registry for shards with a given name
#[doc(hidden)]
#[macro_export]
macro_rules! shard_registry {
    ($vis:vis $registry:ident: $Type:ty) => {
        #[used]
        #[allow(non_upper_case_globals)]
        #[doc(hidden)]
        $vis static $registry: std::sync::Mutex<Vec<$Type>>
                             = std::sync::Mutex::new(Vec::new());
    };
}

/// Executes a block of code before main, by utilising platform specific linker instructions.
#[doc(hidden)]
#[macro_export]
macro_rules! shard_execute_pre_main {
    ($body:expr_2021) => {
        const _: () = {
            #[allow(non_upper_case_globals)]
            #[used]
            // Windows:
            #[cfg_attr(target_os = "windows", unsafe(link_section = ".CRT$XCU"))]
            // macOS + iOS:
            #[cfg_attr(target_os = "ios", unsafe(link_section = "__DATA,__mod_init_func"))]
            #[cfg_attr(target_os = "macos", unsafe(link_section = "__DATA,__mod_init_func"))]
            // Linux, Android, BSD:
            #[cfg_attr(target_os = "android", unsafe(link_section = ".init_array"))]
            #[cfg_attr(target_os = "dragonfly", unsafe(link_section = ".init_array"))]
            #[cfg_attr(target_os = "freebsd", unsafe(link_section = ".init_array"))]
            #[cfg_attr(target_os = "linux", unsafe(link_section = ".init_array"))]
            #[cfg_attr(target_os = "netbsd", unsafe(link_section = ".init_array"))]
            #[cfg_attr(target_os = "openbsd", unsafe(link_section = ".init_array"))]
            // Emscripten
            #[cfg_attr(
                all(target_family = "wasm", target_os = "emscripten"),
                unsafe(link_section = ".init_array")
            )]
            static __init: extern "C" fn() = {
                #[cfg_attr(target_os = "android", unsafe(link_section = ".text.startup"))]
                #[cfg_attr(target_os = "linux", unsafe(link_section = ".text.startup"))]
                extern "C" fn __inner_init() {
                    $body
                }
                __inner_init
            };
        };
    };
}

/// Register a shard to a registry.
#[doc(hidden)]
#[macro_export]
macro_rules! shard_add {
    ( $registry:path; $shard:expr_2021 ) => {
        $crate::shard_execute_pre_main!({
            $registry.lock().unwrap().push($shard);
        });
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! shard_foreach_inner {
    ( $registry:ident; $closure:expr_2021; $( $path_tt:tt )* ) => {
        let guard = $( $path_tt )* $registry
            .lock()
            .unwrap();

        for e in guard.iter() {
            #[allow(clippy::redundant_closure_call)]
            $closure(e);
        }
    };
}

/// Iterate over all shards in unspecified order.
#[doc(hidden)]
#[macro_export]
macro_rules! shard_foreach {
    ( $registry:ident; $closure:expr_2021 ) => {
		$crate::shard_foreach_inner!($registry; $closure; );
	};

    ( $registry:ident in $path:path; $closure:expr_2021 ) => {
		$crate::shard_foreach_inner!($registry; $closure; $path ::);
	};
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    shard_registry!(V: &'static str);

    shard_add!(V; "three");
    shard_add!(V; "four");
    shard_add!(V; "one");
    shard_add!(V; "two");

    #[test]
    fn shard_registry() {
        let expected = HashSet::from(["one", "two", "three", "four"]);
        let mut actual = HashSet::new();

        shard_foreach!(V; |e: &'static str| {
            actual.insert(e);
        });

        assert_eq!(actual, expected);
    }
}
