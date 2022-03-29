use gdext_sys as sys;

/// Co-locates the user's instance (pure Rust) with the Godot "base" object.
///
/// This design does not force the user to keep the base object intrusively in his own struct.
pub(crate) struct InstanceStorage<T> {
    base: sys::GDNativeObjectPtr,
    user_instance: Option<T>, // lateinit
}

impl<T: Default> InstanceStorage<T> {
    fn construct_default(base: sys::GDNativeObjectPtr) -> Self {
        Self {
            base,
            user_instance: T::default(),
        }
    }
}

impl<T> InstanceStorage<T> {
    fn construct(base: sys::GDNativeObjectPtr, user_instance: T) -> Self {
        Self {
            base,
            user_instance,
        }
    }

    #[must_use]
    fn into_ptr(self) -> *mut T {
        Box::into_raw(Box::new(self))
    }
}
