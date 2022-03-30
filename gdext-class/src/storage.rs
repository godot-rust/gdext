use crate::{sys, GodotClass, GodotMethods};

/// Co-locates the user's instance (pure Rust) with the Godot "base" object.
///
/// This design does not force the user to keep the base object intrusively in his own struct.
pub struct InstanceStorage<T: GodotClass> {
    base: sys::GDNativeObjectPtr,
    // base: Obj<T::Base>,
    user_instance: Option<T>, // lateinit
}

impl<T: GodotMethods + GodotClass> InstanceStorage<T> {
    pub fn construct_default(base: sys::GDNativeObjectPtr) -> Self {
        // TODO find a user-friendly repr for base (Obj? T::Base? etc)

        Self {
            //base: unsafe { Obj::<T::Base>::from_sys(base) },
            base,
            user_instance: Some(T::construct(base)),
        }
    }
}

impl<T: GodotClass> InstanceStorage<T> {
    /*pub fn construct(base: sys::GDNativeObjectPtr, user_instance: T) -> Self {
        Self {
            base,
            user_instance: Some(user_instance),
        }
    }*/

    #[must_use]
    pub fn into_raw(self) -> *mut Self {
        Box::into_raw(Box::new(self))
    }

    pub fn get(&self) -> &T {
        self.user_instance
            .as_ref()
            .expect("InstanceStorage not initialized")
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.user_instance
            .as_mut()
            .expect("InstanceStorage not initialized")
    }
}

/// Interprets the opaque pointer as pointing to `InstanceStorage<T>`.
///
/// Note: returns reference with unbounded lifetime; intended for local usage
pub unsafe fn as_storage<'u, T: GodotClass>(
    instance_ptr: *mut std::ffi::c_void,
) -> &'u mut InstanceStorage<T> {
    &mut *(instance_ptr as *mut InstanceStorage<T>)
}
