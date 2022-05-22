/// Adds methods to convert from and to Godot FFI pointers.
#[doc(hidden)]
pub trait GodotFfi {
    type SysPointer;

    /// Construct from Godot opaque pointer.
    unsafe fn from_sys(opaque_ptr: Self::SysPointer) -> Self;

    /// Construct uninitialized opaque data, then initialize it with `init` function.
    unsafe fn from_sys_init(init: impl FnOnce(Self::SysPointer)) -> Self;

    /// Return Godot opaque pointer, for an immutable operation.
    ///
    /// Note that this is a `*mut` pointer despite taking `&self` by shared-ref.
    /// This is because most of Godot's native API is not const-correct. This can still
    /// enhance user code (calling `sys_mut` ensures no aliasing at the time of the call).
    fn sys(&self) -> Self::SysPointer;

    /// Return Godot opaque pointer, for a mutable operation.
    ///
    /// Should usually not be overridden; behaves like `sys()` but ensures no aliasing
    /// at the time of the call (not necessarily during any subsequent modifications though).
    fn sys_mut(&mut self) -> Self::SysPointer {
        self.sys()
    }

    unsafe fn write_sys(&self, dst: Self::SysPointer);
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Macros to choose a certain implementation of `GodotFfi` trait for GDNativeTypePtr;
// or a free-standing `impl` for concrete sys pointers such as GDNativeObjectPtr.

/// Implements FFI methods for a type with `Opaque` data.
/// The sys pointer is directly reinterpreted from/to the `Opaque` and **not** its address.
///
/// Expects a `from_opaque()` constructor and a `opaque` field.
#[macro_export]
macro_rules! impl_ffi_as_opaque {

    // impl GodotFfi for T
    () => {
        type SysPointer = sys::GDNativeTypePtr;
        impl_ffi_as_opaque!(, Self::SysPointer; from_sys, from_sys_init, sys, write_sys);
    };

    // impl T
    ($Ptr:ty; $from_sys:ident, $from_sys_init:ident, $sys:ident, $write_sys:ident) => {
        impl_ffi_as_opaque!(pub, $Ptr; $from_sys, $from_sys_init, $sys, $write_sys);
    };

    // (internal)
    ($vis:vis, $Ptr:ty; $from_sys:ident, $from_sys_init:ident, $sys:ident, $write_sys:ident) => {
        #[doc(hidden)]
        $vis unsafe fn $from_sys(opaque_ptr: $Ptr) -> Self {
            let opaque = std::mem::transmute(opaque_ptr);
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
        type SysPointer = sys::GDNativeTypePtr;
        impl_ffi_as_opaque_pointer!(, Self::SysPointer; from_sys, from_sys_init, sys, write_sys);
    };

    // impl T
    ($Ptr:ty; $from_sys:ident, $from_sys_init:ident, $sys:ident, $write_sys:ident) => {
        impl_ffi_as_opaque_pointer!(pub, $Ptr; $from_sys, $from_sys_init, $sys, $write_sys);
    };

    // (internal)
    ($vis:vis, $Ptr:ty; $from_sys:ident, $from_sys_init:ident, $sys:ident, $write_sys:ident) => {
        #[doc(hidden)]
        $vis unsafe fn $from_sys(opaque_ptr: $Ptr) -> Self {
            let opaque = std::ptr::read(opaque_ptr as *mut _);
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
macro_rules! impl_ffi_as_value {
    // impl GodotFfi for T
    () => {
        type SysPointer = sys::GDNativeTypePtr;
        impl_ffi_as_value!(, Self::SysPointer; from_sys, from_sys_init, sys, write_sys);
    };

    // impl T
    ($Ptr:ty; $from_sys:ident, $from_sys_init:ident, $sys:ident, $write_sys:ident) => {
        impl_ffi_as_value!(pub, $Ptr; $from_sys, $from_sys_init, $sys, $write_sys);
    };

    // (internal)
    ($vis:vis, $Ptr:ty; $from_sys:ident, $from_sys_init:ident, $sys:ident, $write_sys:ident) => {
        $vis unsafe fn $from_sys(opaque_ptr: $Ptr) -> Self {
            *(opaque_ptr as *mut Self)
        }

        $vis unsafe fn $from_sys_init(init: impl FnOnce($Ptr)) -> Self {
            let mut raw = std::mem::MaybeUninit::<Self>::uninit();
            init(raw.as_mut_ptr() as $Ptr);

            raw.assume_init()
        }

        $vis fn sys(&self) -> sys::GDNativeTypePtr {
            self as *const Self as $Ptr
        }

        $vis unsafe fn write_sys(&self, dst: $Ptr) {
            *(dst as *mut Self) = *self;
        }
    };
}
