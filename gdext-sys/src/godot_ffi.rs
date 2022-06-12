use crate as sys;

/// Adds methods to convert from and to Godot FFI pointers.
#[doc(hidden)]
pub trait GodotFfi {
    /// Construct from Godot opaque pointer.
    unsafe fn from_sys(ptr: sys::GDNativeTypePtr) -> Self;

    /// Construct uninitialized opaque data, then initialize it with `init` function.
    unsafe fn from_sys_init(init_fn: impl FnOnce(sys::GDNativeTypePtr)) -> Self;

    /// Return Godot opaque pointer, for an immutable operation.
    ///
    /// Note that this is a `*mut` pointer despite taking `&self` by shared-ref.
    /// This is because most of Godot's native API is not const-correct. This can still
    /// enhance user code (calling `sys_mut` ensures no aliasing at the time of the call).
    fn sys(&self) -> sys::GDNativeTypePtr;

    /// Return Godot opaque pointer, for a mutable operation.
    ///
    /// Should usually not be overridden; behaves like `sys()` but ensures no aliasing
    /// at the time of the call (not necessarily during any subsequent modifications though).
    fn sys_mut(&mut self) -> sys::GDNativeTypePtr {
        self.sys()
    }

    unsafe fn write_sys(&self, dst: sys::GDNativeTypePtr);
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Macros to choose a certain implementation of `GodotFfi` trait for GDNativeTypePtr;
// or a free-standing `impl` for concrete sys pointers such as GDNativeObjectPtr.

/// Implements FFI methods for a type with `Opaque` data.
/// The sys pointer is directly reinterpreted from/to the `Opaque` and **not** its address.
///
/// Expects a `from_opaque()` constructor and a `opaque` field.
#[macro_export]
macro_rules! impl_ffi_as_opaque_value {
    // impl GodotFfi for T
    () => {
        impl_ffi_as_opaque_value!(, gdext_sys::GDNativeTypePtr; from_sys, from_sys_init, sys, write_sys);
    };

    // impl T
    ($Ptr:ty; $from_sys:ident, $from_sys_init:ident, $sys:ident, $write_sys:ident) => {
        impl_ffi_as_opaque_value!(pub, $Ptr; $from_sys, $from_sys_init, $sys, $write_sys);
    };

    // (internal)
    ($vis:vis, $Ptr:ty; $from_sys:ident, $from_sys_init:ident, $sys:ident, $write_sys:ident) => {
        #[doc(hidden)]
        $vis unsafe fn $from_sys(ptr: $Ptr) -> Self {
            let opaque = std::mem::transmute(ptr);
            Self::from_opaque(opaque)
        }

        #[doc(hidden)]
        $vis unsafe fn $from_sys_init(init: impl FnOnce($Ptr)) -> Self {
            let mut raw = std::mem::MaybeUninit::uninit();
            init(std::mem::transmute(raw.as_mut_ptr()));
            Self::from_opaque(raw.assume_init())
        }

        #[doc(hidden)]
        $vis fn $sys(&self) -> $Ptr {
            unsafe { std::mem::transmute(self.opaque) }
        }

        #[doc(hidden)]
        $vis unsafe fn $write_sys(&self, dst: $Ptr) {
            std::ptr::write(dst as *mut _, self.opaque);
        }
    };
}

/// Implements FFI methods for a type with `Opaque` data that stores a value type (e.g. Vector2).
/// The **address of** the `Opaque` field is used as the sys pointer.
///
/// Expects a `from_opaque()` constructor and a `opaque` field.
#[macro_export]
macro_rules! impl_ffi_as_opaque_pointer {
    // impl GodotFfi for T
    () => {
        impl_ffi_as_opaque_pointer!(, $crate::GDNativeTypePtr; from_sys, from_sys_init, sys, write_sys);
    };

    // impl T
    ($Ptr:ty; $from_sys:ident, $from_sys_init:ident, $sys:ident, $write_sys:ident) => {
        impl_ffi_as_opaque_pointer!(pub, $Ptr; $from_sys, $from_sys_init, $sys, $write_sys);
    };

    // (internal)
    ($vis:vis, $Ptr:ty; $from_sys:ident, $from_sys_init:ident, $sys:ident, $write_sys:ident) => {
        #[doc(hidden)]
        $vis unsafe fn $from_sys(ptr: $Ptr) -> Self {
            let opaque = std::ptr::read(ptr as *mut _);
            Self::from_opaque(opaque)
        }

        #[doc(hidden)]
        $vis unsafe fn $from_sys_init(init: impl FnOnce($Ptr)) -> Self {
            let mut raw = std::mem::MaybeUninit::uninit();
            init(raw.as_mut_ptr() as $Ptr);

            Self::from_opaque(raw.assume_init())
        }

        #[doc(hidden)]
        $vis fn $sys(&self) -> $Ptr {
            &self.opaque as *const _ as $Ptr
        }

        #[doc(hidden)]
        $vis unsafe fn $write_sys(&self, dst: $Ptr) {
            // Note: this is the same impl as for impl_ffi_as_opaque_value, which is... interesting
            std::ptr::write(dst as *mut _, self.opaque)
        }
    };
}

/// Implements FFI methods for a type implemented with standard Rust fields (not opaque).
/// The address of `Self` is directly reinterpreted as the sys pointer.
///
/// The size of the corresponding sys type (the `N` in `Opaque*<N>`) must not be bigger than `size_of::<Self>()`.
/// This cannot be checked easily, because Self cannot be used in size_of(). There would of course be workarounds.
#[macro_export]
macro_rules! impl_ffi_as_self_value {
    // impl GodotFfi for T
    () => {
        impl_ffi_as_self_value!(, $crate::GDNativeTypePtr; from_sys, from_sys_init, sys, write_sys);
    };

    // impl T
    ($Ptr:ty; $from_sys:ident, $from_sys_init:ident, $sys:ident, $write_sys:ident) => {
        impl_ffi_as_self_value!(pub, $Ptr; $from_sys, $from_sys_init, $sys, $write_sys);
    };

    // (internal)
    ($vis:vis, $Ptr:ty; $from_sys:ident, $from_sys_init:ident, $sys:ident, $write_sys:ident) => {
        $vis unsafe fn $from_sys(ptr: $Ptr) -> Self {
            *(ptr as *mut Self)
        }

        $vis unsafe fn $from_sys_init(init: impl FnOnce($Ptr)) -> Self {
            let mut raw = std::mem::MaybeUninit::<Self>::uninit();
            init(raw.as_mut_ptr() as $Ptr);

            raw.assume_init()
        }

        $vis fn sys(&self) -> $Ptr {
            self as *const Self as $Ptr
        }

        $vis unsafe fn write_sys(&self, dst: $Ptr) {
            *(dst as *mut Self) = *self;
        }
    };
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation for common types (needs to be this crate due to orphan rule)
mod scalars {
    use super::GodotFfi;
    use crate as sys;

    macro_rules! impl_godot_ffi {
        ($T:ty) => {
            impl GodotFfi for $T {
                impl_ffi_as_self_value!();
            }
        };
    }

    impl_godot_ffi!(bool);
    impl_godot_ffi!(i64);
    impl_godot_ffi!(i32); // FIXME remove
    impl_godot_ffi!(f64);

    impl GodotFfi for () {
        unsafe fn from_sys(_ptr: sys::GDNativeTypePtr) -> Self {
            // Do nothing
        }

        unsafe fn from_sys_init(_init: impl FnOnce(sys::GDNativeTypePtr)) -> Self {
            // Do nothing
        }

        fn sys(&self) -> sys::GDNativeTypePtr {
            // ZST dummy pointer
            self as *const _ as sys::GDNativeTypePtr
        }

        unsafe fn write_sys(&self, _dst: sys::GDNativeTypePtr) {
            // Do nothing
        }
    }
}
