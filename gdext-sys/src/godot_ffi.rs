/// Adds methods to convert from and to Godot FFI pointers.
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

/// Implements the `GodotFfi` methods for a type with `Opaque` data that stores a pointer type
/// (e.g. string, variant).
///
/// Expects a `from_opaque()` constructor and a `opaque` field.
// TODO make sure this whole thing is correct, especially from_sys_init()
#[macro_export]
macro_rules! impl_ffi_as_opaque_inplace_pointer {
    ($Ptr:ty) => {
        type SysPointer = $Ptr;

        unsafe fn from_sys(opaque_ptr: $Ptr) -> Self {
            debug_assert!(!opaque_ptr.is_null());

            let opaque = std::mem::transmute(opaque_ptr);
            Self::from_opaque(opaque)
        }

        unsafe fn from_sys_init(init: impl FnOnce($Ptr)) -> Self {
            let mut raw = std::mem::MaybeUninit::uninit();
            // init(std::ptr::read(
            //     raw.as_mut_ptr() as *mut sys::GDNativeTypePtr
            // ));

            init(std::mem::transmute(raw.as_mut_ptr())); // <- this was the OpaqueObject::with_init() version
                                                         //init(std::mem::transmute(raw));

            Self::from_opaque(raw.assume_init())
        }

        fn sys(&self) -> $Ptr {
            unsafe { std::mem::transmute(self.opaque) }
        }

        unsafe fn write_sys(&self, dst: $Ptr) {
            std::ptr::write(dst as *mut _, self.opaque);
        }
    };
}

/// Implements the `GodotFfi` methods for a type with `Opaque` data that stores a value type
/// (e.g. variant, vector2).
///
/// Expects a `from_opaque()` constructor and a `opaque` field.
#[macro_export]
macro_rules! impl_ffi_as_opaque_pointer {
    ($Ptr:ty) => {
        type SysPointer = $Ptr;

        unsafe fn from_sys(opaque_ptr: $Ptr) -> Self {
            let opaque = std::ptr::read(opaque_ptr as *mut _);
            Self::from_opaque(opaque)
        }

        unsafe fn from_sys_init(init: impl FnOnce($Ptr)) -> Self {
            let mut raw = std::mem::MaybeUninit::uninit();
            init(raw.as_mut_ptr() as $Ptr);

            Self::from_opaque(raw.assume_init())
        }

        fn sys(&self) -> $Ptr {
            &self.opaque as *const _ as $Ptr
        }

        unsafe fn write_sys(&self, dst: $Ptr) {
            std::ptr::write(dst as *mut _, self.opaque)
        }
    };
}

/// Implements the `GodotFfi` methods for a type implemented with standard Rust fields
/// (not opaque).
///
/// The size of the corresponding sys type (the `N` in `Opaque*<N>`) must not be bigger than `size_of::<Self>()`.
/// This cannot be checked easily, because Self cannot be used in size_of(). There would of course be workarounds.
#[macro_export]
macro_rules! impl_ffi_as_value {
    () => {
        type SysPointer = sys::GDNativeTypePtr;

        unsafe fn from_sys(opaque_ptr: sys::GDNativeTypePtr) -> Self {
            *(opaque_ptr as *mut Self)
        }

        unsafe fn from_sys_init(init: impl FnOnce(sys::GDNativeTypePtr)) -> Self {
            let mut raw = std::mem::MaybeUninit::<Self>::uninit();
            init(raw.as_mut_ptr() as sys::GDNativeTypePtr);

            raw.assume_init()
        }

        fn sys(&self) -> sys::GDNativeTypePtr {
            self as *const Self as sys::GDNativeTypePtr
        }

        unsafe fn write_sys(&self, dst: sys::GDNativeTypePtr) {
            *(dst as *mut Self) = *self;
        }
    };
}
