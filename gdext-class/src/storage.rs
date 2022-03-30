use crate::sys;

/// Co-locates the user's instance (pure Rust) with the Godot "base" object.
///
/// This design does not force the user to keep the base object intrusively in his own struct.
pub struct InstanceStorage<T> {
    base: sys::GDNativeObjectPtr,
    user_instance: Option<T>, // lateinit
}

impl<T: Default> InstanceStorage<T> {
    pub fn construct_default(base: sys::GDNativeObjectPtr) -> Self {
        Self {
            base,
            user_instance: Some(T::default()),
        }
    }
}

impl<T> InstanceStorage<T> {
    pub fn construct(base: sys::GDNativeObjectPtr, user_instance: T) -> Self {
        Self {
            base,
            user_instance: Some(user_instance),
        }
    }

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
pub unsafe fn as_storage<'u, T>(instance_ptr: *mut std::ffi::c_void) -> &'u mut InstanceStorage<T> {
    &mut *(instance_ptr as *mut InstanceStorage<T>)
}
