use crate::{out, sys, Base, GodotClass, GodotDefault, Obj};
use std::any::type_name;
use std::mem;

/// Co-locates the user's instance (pure Rust) with the Godot "base" object.
///
/// This design does not force the user to keep the base object intrusively in his own struct.
pub struct InstanceStorage<T: GodotClass> {
    // Raw pointer, but can be converted to Obj<T::Base> at most once; then becomes "used" (null)
    // This is done lazily to avoid taking ownership (potentially increasing ref-pointer)
    base_ptr: sys::GDNativeObjectPtr,

    // lateinit; see get_mut_lateinit()
    // FIXME should be RefCell, to avoid multi-aliasing (mut borrows from multiple shared Obj<T>)
    user_instance: Option<T>,

    // Declared after `user_instance`, is dropped last
    pub lifecycle: Lifecycle,
    godot_ref_count: i32,

    _last_drop: LastDrop,
}

#[derive(Copy, Clone, Debug)]
pub enum Lifecycle {
    Initializing,
    Alive,
    Destroying,
    Dead, // reading this would typically already be too late, only best-effort in case of UB
}

struct LastDrop;
impl Drop for LastDrop {
    fn drop(&mut self) {
        println!("LAST DROP");
    }
}

impl<T: GodotDefault + GodotClass> InstanceStorage<T> {
    pub fn initialize_default(&mut self) {
        out!("    Storage::initialize_default  <{}>", type_name::<T>());

        let base = Self::consume_base(&mut self.base_ptr);
        self.initialize(T::construct(base));
    }

    pub fn get_mut_lateinit(&mut self) -> &mut T {
        out!("    Storage::get_mut_lateinit      <{}>", type_name::<T>());

        // We need to provide lazy initialization for ptrcalls and varcalls coming from the engine.
        // The `create_instance_func` callback cannot know yet how to initialize the instance (a user
        // could provide an initial value, or use default construction). Since this method is used
        // for both construction from Rust (through Obj) and from GDScript (through T.new()), this
        // initializes the value lazily.
        let result = self.user_instance.get_or_insert_with(|| {
            out!("    Storage::lateinit              <{}>", type_name::<T>());

            let base = Self::consume_base(&mut self.base_ptr);
            T::construct(base)
        });

        assert!(matches!(
            self.lifecycle,
            Lifecycle::Initializing | Lifecycle::Alive
        ));
        self.lifecycle = Lifecycle::Alive;

        result
    }
}

impl<T: GodotClass> InstanceStorage<T> {
    pub fn construct_uninit(base: sys::GDNativeObjectPtr) -> Self {
        out!("    Storage::construct_uninit      <{}>", type_name::<T>());

        Self {
            base_ptr: base,
            user_instance: None,
            lifecycle: Lifecycle::Initializing,
            godot_ref_count: 1,
            _last_drop: LastDrop,
        }
    }

    pub fn initialize(&mut self, value: T) {
        out!("    Storage::initialize          <{}>", type_name::<T>());
        assert!(
            self.user_instance.is_none(),
            "Cannot initialize user instance multiple times"
        );
        assert!(matches!(self.lifecycle, Lifecycle::Initializing));

        self.user_instance = Some(value);
        self.lifecycle = Lifecycle::Alive;
    }

    pub(crate) fn on_inc_ref(&mut self) {
        self.godot_ref_count += 1;
        out!(
            "    Storage::on_inc_ref (rc={})     <{}> -- {:?}",
            self.godot_ref_count,
            type_name::<T>(),
            self.user_instance
        );
    }

    pub(crate) fn on_dec_ref(&mut self) {
        self.godot_ref_count -= 1;
        out!(
            "  | Storage::on_dec_ref (rc={})     <{}> -- {:?}",
            self.godot_ref_count,
            type_name::<T>(),
            self.user_instance
        );
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

    pub fn mark_destroyed_by_godot(&mut self) {
        out!(
            "    Storage::mark_destroyed_by_godot -- {:?}",
            self.user_instance
        );
        self.lifecycle = Lifecycle::Destroying;
        out!(
            "    mark;  self={:?}, val={:?}",
            self as *mut _,
            self.lifecycle
        );
    }

    #[inline(always)]
    pub fn destroyed_by_godot(&self) -> bool {
        out!(
            "    is_d;  self={:?}, val={:?}",
            self as *const _,
            self.lifecycle
        );
        matches!(self.lifecycle, Lifecycle::Destroying | Lifecycle::Dead)
    }

    // Note: not &mut self, to only borrow one field and not the entire struct
    fn consume_base(base_ptr: &mut sys::GDNativeObjectPtr) -> Base<T::Base> {
        // Check that this method is called at most once
        assert!(
            !base_ptr.is_null(),
            "Instance base has already been consumed"
        );

        let base = mem::replace(base_ptr, std::ptr::null_mut());
        let obj = unsafe { Obj::from_obj_sys(base) };

        // This object does not contribute to the strong count, otherwise we create a reference cycle:
        // 1. RefCounted (dropped in GDScript)
        // 2. holds user T (via extension instance and storage)
        // 3. holds #[base] RefCounted (last ref, dropped in T destructor, but T is never destroyed because this ref keeps storage alive)
        // Note that if late-init never happened on self, we have the same behavior (still a raw pointer instead of weak Obj)
        Base::from_obj(obj)
    }
}

impl<T: GodotClass> Drop for InstanceStorage<T> {
    fn drop(&mut self) {
        out!(
            "    Storage::drop (rc={})           <{}> -- {:?}",
            self.godot_ref_count,
            type_name::<T>(),
            self.user_instance
        );
        //let _ = mem::take(&mut self.user_instance);
        out!(
            "    Storage::drop end              <{}>  -- {:?}",
            type_name::<T>(),
            self.user_instance
        );
    }
}

/// Interprets the opaque pointer as pointing to `InstanceStorage<T>`.
///
/// Note: returns reference with unbounded lifetime; intended for local usage
// FIXME unbounded ref AND &mut out of thin air is a huge hazard -- consider using with_storage(ptr, closure) and drop_storage(ptr)
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
