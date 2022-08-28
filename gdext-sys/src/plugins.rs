/// Distributed self-registration of "plugins" without central list

// Note: code in this file is safe, however it seems that some annotations fall into the "unsafe" category.
// For example, adding #![forbid(unsafe_code)] causes this error:
//   note: the program's behavior with overridden link sections on items is unpredictable
//   and Rust cannot provide guarantees when you manually override them

/// Declare a global registry for plugins with a given name
#[doc(hidden)]
#[macro_export]
macro_rules! plugin_registry {
    ($registry:ident: $Type:ty) => {
        $crate::paste::paste! {
            #[used]
            #[allow(non_upper_case_globals)]
            #[doc(hidden)]
            pub static [< __godot_rust_plugin_ $registry >]:
                std::sync::Mutex<Vec<$Type>> = std::sync::Mutex::new(Vec::new());
        }
    };
}

/// Register a plugin to a registry
#[doc(hidden)]
#[macro_export]
#[rustfmt::skip] // paste's [< >] syntax chokes fmt
macro_rules! plugin_add {
    ( $registry:ident; $plugin:expr ) => {
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
                	let mut guard = $crate::paste::paste!( [< __godot_rust_plugin_ $registry >] )
                        .lock()
                        .unwrap();
                    guard.push($plugin);
                }
                __inner_init
            };
        };
    };



	 ( $($qual:ident ::)+ ; $registry:ident; $plugin:expr ) => {
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
                	let mut guard =  $crate::paste::paste!( $($qual ::)+ [< __godot_rust_plugin_ $registry >] )
                        .lock()
                        .unwrap();
                    guard.push($plugin);
                }
                __inner_init
            };
        };
    };
}

/// Iterate over all plugins in unspecified order
#[doc(hidden)]
#[macro_export]
macro_rules! plugin_foreach {
    ($registry:ident; $closure:expr) => {
        let guard = $crate::paste::paste!( [< __godot_rust_plugin_ $registry >] )
            .lock()
            .unwrap();

        for e in guard.iter() {
            $closure(e);
        }
    };
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[cfg(test)]
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
