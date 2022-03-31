use crate::{out, sys, GodotClass, GodotMethods};

/// Co-locates the user's instance (pure Rust) with the Godot "base" object.
///
/// This design does not force the user to keep the base object intrusively in his own struct.
pub struct InstanceStorage<T: GodotClass> {
    base: sys::GDNativeObjectPtr,
    // base: Obj<T::Base>,
    user_instance: Option<T>, // lateinit
    refcount: i32,
}

impl<T: GodotMethods + GodotClass> InstanceStorage<T> {
    pub fn construct_default(base: sys::GDNativeObjectPtr) -> Self {
        // TODO find a user-friendly repr for base (Obj? T::Base? etc)
        let instance = T::construct(base);

        let refcount = 1;
        out!(
            "[Storage] construct_def: {:?}\n  refcount: {}",
            instance,
            refcount
        );
        Self {
            //base: unsafe { Obj::<T::Base>::from_sys(base) },
            base,
            user_instance: Some(instance),
            refcount,
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

    pub(crate) fn inc_ref(&mut self) {
        self.refcount += 1;
        out!(
            "[Storage] inc_ref: {:?}\n  refcount: {}",
            self.get(),
            self.refcount
        );
    }

    pub(crate) fn dec_ref(&mut self) {
        self.refcount -= 1;
        out!(
            "[Storage] dec_ref: {:?}\n  refcount: {}",
            self.get(),
            self.refcount
        );
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

impl<T: GodotClass> Drop for InstanceStorage<T> {
    fn drop(&mut self) {
        out!(
            "[Storage] drop: {:?}\n  refcount: {}",
            self.get(),
            self.refcount
        );
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
