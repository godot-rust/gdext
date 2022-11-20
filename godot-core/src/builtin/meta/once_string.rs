use crate::builtin::StringName;
use std::cell::Cell;
use std::mem::ManuallyDrop;

use crate::builtin::meta::PropertyInfo;
use godot_ffi as sys;

/// Utility to safely pass strings to FFI without creating dangling pointers or memory leaks.
///
/// Models a recurring pattern when passing strings to FFI functions:
/// ```no_run
/// # fn some_ffi_function(_ptr: godot_ffi::GDNativeStringNamePtr) {}
/// # use godot_core::builtin::StringName;
/// let s: StringName = todo!();
///
/// // Pass pointer to Godot FFI -- the underlying object must remain valid, because the sys pointer
/// // might point to this object. So we can't use a .leak_sys() function which consumes self.
/// some_ffi_function(s.string_sys());
///
/// // We don't own the string anymore, so we must not destroy it
/// std::mem::forget(s);
///
/// // However, if in fact string_sys() was never invoked (e.g. behind an if),
/// // we now have a memory leak...
/// ```
#[doc(hidden)]
pub struct OnceArg<T: OnceSys> {
    obj: ManuallyDrop<T>,
    has_ownership: Cell<bool>,
}

#[doc(hidden)]
pub type OnceString = OnceArg<StringName>;

impl OnceString {
    pub fn new(rust_string: &str) -> Self {
        Self::from_owned(StringName::from(rust_string))
    }
}

impl<T: OnceSys> OnceArg<T> {
    pub fn from_owned(value: T) -> Self {
        Self {
            obj: ManuallyDrop::new(value),
            has_ownership: Cell::new(true),
        }
    }

    /// Access sys pointer while retaining underlying referred-to object.
    ///
    /// Can only be called once. Leaks memory if unused.
    #[must_use]
    pub fn leak_sys(&self) -> T::SysPointer {
        // Interior mutability because this semantically acts like a consuming `self` function, which doesn't require `mut` either.

        let had_ownership = self.has_ownership.replace(false);
        assert!(had_ownership, "cannot call leak_sys() more than once");

        self.obj.once_sys()
    }
}

impl<T: OnceSys> Drop for OnceArg<T> {
    fn drop(&mut self) {
        // If no one called leak, destroy normally to avoid memory leaks
        if self.has_ownership.get() {
            unsafe { ManuallyDrop::drop(&mut self.obj) };
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[doc(hidden)]
pub trait OnceSys {
    type SysPointer;

    fn once_sys(&self) -> Self::SysPointer;
}

impl OnceSys for StringName {
    type SysPointer = sys::GDNativeStringNamePtr;

    fn once_sys(&self) -> Self::SysPointer {
        self.string_sys()
    }
}

impl OnceSys for PropertyInfo {
    type SysPointer = sys::GDNativePropertyInfo;

    fn once_sys(&self) -> Self::SysPointer {
        self.property_sys()
    }
}
