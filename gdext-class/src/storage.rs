use crate::{out, sys, DefaultConstructible, GodotClass, Obj};

/// Co-locates the user's instance (pure Rust) with the Godot "base" object.
///
/// This design does not force the user to keep the base object intrusively in his own struct.
pub struct InstanceStorage<T: GodotClass> {
    // Raw pointer, but can be converted to Obj<T::Base> at most once; then becomes "used" (null)
    // This is done lazily to avoid taking ownership (potentially increasing ref-pointer)
    base_ptr: sys::GDNativeObjectPtr,

    // lateinit; see get_mut_lateinit()
    user_instance: Option<T>,
}

impl<T: DefaultConstructible + GodotClass> InstanceStorage<T> {
    pub fn initialize_default(&mut self) {
        let base = self.consume_base();
        self.initialize(T::construct(base));
    }

    pub fn get_mut_lateinit(&mut self) -> &mut T {
        // We need to provide lazy initialization for ptrcalls and varcalls coming from the engine.
        // The `create_instance_func` callback cannot know yet how to initialize the instance (a user
        // could provide an initial value, or use default construction). Since this method is used
        // for both construction from Rust (through Obj) and from GDScript (through T.new()), this
        // initializes the value lazily.
        let base = self.consume_base();
        self.user_instance
            .get_or_insert_with(|| T::construct(base))
    }
}

impl<T: GodotClass> InstanceStorage<T> {
    pub fn construct_uninit(base: sys::GDNativeObjectPtr) -> Self {
        // let refcount = 1;
        //out!("[Storage] construct_uninit:  refcount: {}", refcount);

        Self {
            base_ptr: base,
            user_instance: None,
        }
    }

    pub fn initialize(&mut self, value: T) {
        assert!(
            self.user_instance.is_none(),
            "Cannot initialize user instance multiple times"
        );

        self.user_instance = Some(value);
    }

    pub(crate) fn on_inc_ref(&mut self) {
        // self.refcount += 1;
        out!("    Storage::on_inc_ref -- {:?}", self.get());
    }

    pub(crate) fn on_dec_ref(&mut self) {
        // self.refcount -= 1;
        out!("  | Storage::on_dec_ref -- {:?}", self.get());
    }

    /* pub fn destroy(&mut self) {
        assert!(
            self.user_instance.is_some(),
            "Cannot destroy user instance which is not yet initialized"
        );
        assert!(
            !self.destroyed,
            "Cannot destroy user instance multiple times"
        );
        self.user_instance = None; // drops T
                                   // TODO drop entire Storage
    }*/

    #[must_use]
    pub fn into_raw(self) -> *mut Self {
        Box::into_raw(Box::new(self))
    }

    pub fn get(&self) -> &T {
        self.user_instance
            .as_ref()
            .expect("get(): user instance not initialized")
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.user_instance
            .as_mut()
            .expect("get_mut(): user instance not initialized")
    }

    fn consume_base(&mut self) -> Obj<T::Base> {
        // Check that this method is called at most once
        assert!(
            !self.base_ptr.is_null(),
            "Instance base has already been consumed"
        );
        let base = std::mem::replace(&mut self.base_ptr, std::ptr::null_mut());
        unsafe { Obj::from_obj_sys(base) }
    }
}

impl<T: GodotClass> Drop for InstanceStorage<T> {
    fn drop(&mut self) {
        out!("    Storage::drop -- {:?}", self.user_instance);
    }
}

/// Interprets the opaque pointer as pointing to `InstanceStorage<T>`.
///
/// Note: returns reference with unbounded lifetime; intended for local usage
pub unsafe fn as_storage<'u, T: GodotClass>(
    instance_ptr: sys::GDExtensionClassInstancePtr,
) -> &'u mut InstanceStorage<T> {
    &mut *(instance_ptr as *mut InstanceStorage<T>)
}

pub fn nop_instance_callbacks() -> sys::GDNativeInstanceBindingCallbacks {
    // These could also be null pointers, if they are definitely not invoked (e.g. create_callback only passed to object_get_instance_binding(),
    // when there is already a binding). Current "empty but not null" impl corresponds to godot-cpp (wrapped.hpp).
    sys::GDNativeInstanceBindingCallbacks {
        create_callback: Some(create_callback),
        free_callback: Some(free_callback),
        reference_callback: Some(reference_callback),
    }
}

extern "C" fn create_callback(
    _p_token: *mut std::os::raw::c_void,
    _p_instance: *mut std::os::raw::c_void,
) -> *mut std::os::raw::c_void {
    // There is no "instance binding" for Godot types like Node3D -- this would be the user-defined Rust class
    std::ptr::null_mut()
}

extern "C" fn free_callback(
    _p_token: *mut std::os::raw::c_void,
    _p_instance: *mut std::os::raw::c_void,
    _p_binding: *mut std::os::raw::c_void,
) {
}

extern "C" fn reference_callback(
    _p_token: *mut std::os::raw::c_void,
    _p_binding: *mut std::os::raw::c_void,
    _p_reference: sys::GDNativeBool,
) -> sys::GDNativeBool {
    true as u8
}
