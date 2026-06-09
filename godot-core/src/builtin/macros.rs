/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#![macro_use]

// Trait bodies parametrized on the lifecycle accessor `$lifecycle`: the main-thread table or the reviewed thread-safe subset. See
// `impl_builtin_traits!` for how the accessor is chosen.
macro_rules! impl_builtin_traits_inner {
    ( Default for $Type:ty => $gd_method:ident, $lifecycle:path ) => {
        impl Default for $Type {
            #[inline]
            fn default() -> Self {
                unsafe {
                    Self::new_with_uninit(|self_ptr| {
                        ($lifecycle().$gd_method)(self_ptr, std::ptr::null());
                    })
                }
            }
        }
    };

    ( Clone for $Type:ty => $gd_method:ident, $lifecycle:path ) => {
        impl Clone for $Type {
            #[inline]
            fn clone(&self) -> Self {
                unsafe {
                    Self::new_with_uninit(|self_ptr| {
                        let args = [self.sys()];
                        ($lifecycle().$gd_method)(self_ptr, args.as_ptr());
                    })
                }
            }
        }
    };

    ( Drop for $Type:ty => $gd_method:ident, $lifecycle:path ) => {
        impl Drop for $Type {
            #[inline]
            fn drop(&mut self) {
                unsafe {
                    ($lifecycle().$gd_method)(self.sys_mut());
                }
            }
        }
    };

    ( PartialEq for $Type:ty => $gd_method:ident, $lifecycle:path ) => {
        impl PartialEq for $Type {
            #[inline]
            fn eq(&self, other: &Self) -> bool {
                unsafe {
                    let mut result = false;
                    ($lifecycle().$gd_method)(self.sys(), other.sys(), result.sys_mut());
                    result
                }
            }
        }
    };

    ( Eq for $Type:ty => $gd_method:ident, $lifecycle:path ) => {
        impl_builtin_traits_inner!(PartialEq for $Type => $gd_method, $lifecycle);
        impl Eq for $Type {}
    };

    ( Ord for $Type:ty => $gd_method:ident, $lifecycle:path ) => {
        impl Ord for $Type {
            #[inline]
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                let op_less = |lhs, rhs| unsafe {
                    let mut result = false;
                    ($lifecycle().$gd_method)(lhs, rhs, result.sys_mut());
                    result
                };

                if op_less(self.sys(), other.sys()) {
                    std::cmp::Ordering::Less
                } else if op_less(other.sys(), self.sys()) {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            }
        }
        impl PartialOrd for $Type {
            #[inline]
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }
    };

    // Hash is pure Rust-side (no FFI), so it ignores the lifecycle accessor.
    ( Hash for $Type:ty, $lifecycle:path ) => {
        impl std::hash::Hash for $Type {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                // The GDExtension interface only deals in `int64_t`, but the engine's own `hash()` function
                // actually returns `uint32_t`.
                self.hash_u32().hash(state)
            }
        }
    };
}

macro_rules! impl_builtin_traits {
    (
        for $Type:ty {
            $( $Trait:ident $(=> $gd_method:ident)?; )*
        }
    ) => (
        $(
            impl_builtin_traits_inner! {
                $Trait for $Type $(=> $gd_method)?, ::godot_ffi::builtin_lifecycle_api
            }
        )*
    );

    // Thread-safe variant: routes through the reviewed `sys::thread_safe_lifecycle()` subset. Only traits whose lifecycle functions are in
    // that subset may be listed; anything else is a compile error on the missing field. Additionally, $Type must implement Send.
    (
        thread_safe for $Type:ty {
            $( $Trait:ident $(=> $gd_method:ident)?; )*
        }
    ) => (
        const _: () = ::godot_ffi::require_send::<$Type>();
        $(
            impl_builtin_traits_inner! {
                $Trait for $Type $(=> $gd_method)?, ::godot_ffi::thread_safe_lifecycle
            }
        )*
    )
}
