/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#[cfg(debug_assertions)]
use std::cell::Cell;
use std::cell::RefCell;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::mem::ManuallyDrop;
use std::rc::Rc;

use crate::builtin::{Callable, Variant};
use crate::obj::{bounds, Gd, GodotClass};
use crate::{classes, sys};

/// Represents the initialization state of a `Base<T>` object.
#[cfg(debug_assertions)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum InitState {
    /// Object is being constructed (inside `I*::init()` or `Gd::from_init_fn()`).
    ObjectConstructing,
    /// Object construction is complete.
    ObjectInitialized,
    /// `ScriptInstance` context - always considered initialized (bypasses lifecycle checks).
    Script,
}

#[cfg(debug_assertions)]
macro_rules! base_from_obj {
    ($obj:expr, $state:expr) => {
        Base::from_obj($obj, $state)
    };
}

#[cfg(not(debug_assertions))]
macro_rules! base_from_obj {
    ($obj:expr, $state:expr) => {
        Base::from_obj($obj)
    };
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Restricted version of `Gd`, to hold the base instance inside a user's `GodotClass`.
///
/// Behaves similarly to [`Gd`][crate::obj::Gd], but is more constrained. Cannot be constructed by the user.
pub struct Base<T: GodotClass> {
    // Like `Gd`, it's theoretically possible that Base is destroyed while there are still other Gd pointers to the underlying object. This is
    // safe, may however lead to unintended behavior. The base_test.rs file checks some of these scenarios.

    // Internal smart pointer is never dropped. It thus acts like a weak pointer and is needed to break reference cycles between Gd<T>
    // and the user instance owned by InstanceStorage.
    //
    // There is no data apart from the opaque bytes, so no memory or resources to deallocate.
    // When triggered by Godot/GDScript, the destruction order is as follows:
    // 1.    Most-derived Godot class (C++)
    //      ...
    // 2.  RefCounted (C++)
    // 3. Object (C++) -- this triggers InstanceStorage destruction
    // 4.   Base<T>
    // 5.  User struct (GodotClass implementation)
    // 6. InstanceStorage
    //
    // When triggered by Rust (Gd::drop on last strong ref), it's as follows:
    // 1.   Gd<T>  -- triggers InstanceStorage destruction
    // 2.
    obj: ManuallyDrop<Gd<T>>,

    /// Additional strong ref, needed to prevent destruction if [`Self::to_init_gd()`] is called on ref-counted objects.
    extra_strong_ref: Rc<RefCell<Option<Gd<T>>>>,

    /// Tracks the initialization state of this `Base<T>` in Debug mode.
    ///
    /// Rc allows to "copy-construct" the base from an existing one, while still affecting the user-instance through the original `Base<T>`.
    #[cfg(debug_assertions)]
    init_state: Rc<Cell<InitState>>,
}

impl<T: GodotClass> Base<T> {
    /// "Copy constructor": allows to share a `Base<T>` weak pointer.
    ///
    /// The return value is a weak pointer, so it will not keep the instance alive.
    ///
    /// # Safety
    /// `base` must be alive at the time of invocation, i.e. user `init()` (which could technically destroy it) must not have run yet.
    /// If `base` is destroyed while the returned `Base<T>` is in use, that constitutes a logic error, not a safety issue.
    pub(crate) unsafe fn from_base(base: &Base<T>) -> Base<T> {
        debug_assert!(base.obj.is_instance_valid());

        let obj = Gd::from_obj_sys_weak(base.obj.obj_sys());

        Self {
            obj: ManuallyDrop::new(obj),
            extra_strong_ref: Rc::clone(&base.extra_strong_ref), // Before user init(), no handing out of Gd pointers occurs.
            #[cfg(debug_assertions)]
            init_state: Rc::clone(&base.init_state),
        }
    }

    /// Create base from existing object (used in script instances).
    ///
    /// The return value is a weak pointer, so it will not keep the instance alive.
    ///
    /// # Safety
    /// `gd` must be alive at the time of invocation. If it is destroyed while the returned `Base<T>` is in use, that constitutes a logic
    /// error, not a safety issue.
    pub(crate) unsafe fn from_script_gd(gd: &Gd<T>) -> Self {
        debug_assert!(gd.is_instance_valid());

        let obj = Gd::from_obj_sys_weak(gd.obj_sys());
        base_from_obj!(obj, InitState::Script)
    }

    /// Create new base from raw Godot object.
    ///
    /// The return value is a weak pointer, so it will not keep the instance alive.
    ///
    /// # Safety
    /// `base_ptr` must point to a valid, live object at the time of invocation. If it is destroyed while the returned `Base<T>` is in use,
    /// that constitutes a logic error, not a safety issue.
    pub(crate) unsafe fn from_sys(base_ptr: sys::GDExtensionObjectPtr) -> Self {
        assert!(!base_ptr.is_null(), "instance base is null pointer");

        // Initialize only as weak pointer (don't increment reference count).
        let obj = Gd::from_obj_sys_weak(base_ptr);

        // This obj does not contribute to the strong count, otherwise we create a reference cycle:
        // 1. RefCounted (dropped in GDScript)
        // 2. holds user T (via extension instance and storage)
        // 3. holds Base<T> RefCounted (last ref, dropped in T destructor, but T is never destroyed because this ref keeps storage alive)
        // Note that if late-init never happened on self, we have the same behavior (still a raw pointer instead of weak Gd)
        base_from_obj!(obj, InitState::ObjectConstructing)
    }

    #[cfg(debug_assertions)]
    fn from_obj(obj: Gd<T>, init_state: InitState) -> Self {
        Self {
            obj: ManuallyDrop::new(obj),
            extra_strong_ref: Rc::new(RefCell::new(None)),
            init_state: Rc::new(Cell::new(init_state)),
        }
    }

    #[cfg(not(debug_assertions))]
    fn from_obj(obj: Gd<T>) -> Self {
        Self {
            obj: ManuallyDrop::new(obj),
            extra_strong_ref: Rc::new(RefCell::new(None)),
        }
    }

    /// Returns a [`Gd`] referencing the same object as this reference.
    ///
    /// Using this method to call methods on the base field of a Rust object is discouraged, instead use the
    /// methods from [`WithBaseField`](super::WithBaseField) when possible.
    #[doc(hidden)]
    #[deprecated = "Private API. Use `Base::to_init_gd()` or `WithBaseField::to_gd()` instead."] // TODO(v0.4): remove.
    pub fn to_gd(&self) -> Gd<T> {
        (*self.obj).clone()
    }

    /// Returns a [`Gd`] referencing the base object, for exclusive use during object initialization.
    ///
    /// Can be used during an initialization function [`I*::init()`][crate::classes::IObject::init] or [`Gd::from_init_fn()`].
    ///
    /// The base pointer is only pointing to a base object; you cannot yet downcast it to the object being constructed.
    /// The instance ID is the same as the one the in-construction object will have.
    ///
    /// # Lifecycle for ref-counted classes
    /// If `T: Inherits<RefCounted>`, then the ref-counted object is not yet fully-initialized at the time of the `init` function running.
    /// Accessing the base object without further measures would be dangerous. Here, godot-rust employs a workaround: the `Base` object (which
    /// holds a weak pointer to the actual instance) is temporarily upgraded to a strong pointer, preventing use-after-free.
    ///
    /// This additional reference is automatically dropped at an implementation-defined point in time (which may change, and technically delay
    /// destruction of your object as soon as you use `Base::to_init_gd()`). Right now, this refcount-decrement is deferred to the next frame.
    ///
    /// For now, ref-counted bases can only use `to_init_gd()` on the main thread.
    ///
    /// # Panics (Debug)
    /// If called outside an initialization function, or for ref-counted objects on a non-main thread.
    #[cfg(since_api = "4.2")]
    pub fn to_init_gd(&self) -> Gd<T> {
        #[cfg(debug_assertions)] // debug_assert! still checks existence of symbols.
        assert!(
            self.is_initializing(),
            "Base::to_init_gd() can only be called during object initialization, inside I*::init() or Gd::from_init_fn()"
        );

        // For manually-managed objects, regular clone is fine.
        // Only static type matters, because this happens immediately after initialization, so T is both static and dynamic type.
        if !<T::Memory as bounds::Memory>::IS_REF_COUNTED {
            return Gd::clone(&self.obj);
        }

        debug_assert!(
            sys::is_main_thread(),
            "Base::to_init_gd() can only be called on the main thread for ref-counted objects (for now)"
        );

        // First time handing out a Gd<T>, we need to take measures to temporarily upgrade the Base's weak pointer to a strong one.
        // During the initialization phase (derived object being constructed), increment refcount by 1.
        if self.extra_strong_ref.borrow().is_none() {
            let strong_ref = unsafe { Gd::from_obj_sys(self.obj.obj_sys()) };
            *self.extra_strong_ref.borrow_mut() = Some(strong_ref);
        }

        // Can't use Gd::apply_deferred(), as that implicitly borrows &mut self, causing a "destroyed while bind was active" panic.
        let name = format!("Base<{}> deferred unref", T::class_name());
        let rc = Rc::clone(&self.extra_strong_ref);
        let callable = Callable::from_once_fn(&name, move |_args| {
            Self::drop_strong_ref(rc);
            Ok(Variant::nil())
        });
        callable.call_deferred(&[]);

        (*self.obj).clone()
    }

    /// Drops any extra strong references, possibly causing object destruction.
    fn drop_strong_ref(extra_strong_ref: Rc<RefCell<Option<Gd<T>>>>) {
        let mut r = extra_strong_ref.borrow_mut();
        assert!(r.is_some());

        *r = None; // Triggers RawGd::drop() -> dec-ref -> possibly object destruction.
    }

    /// Finalizes the initialization of this `Base<T>` and returns whether
    pub(crate) fn mark_initialized(&mut self) {
        #[cfg(debug_assertions)]
        {
            assert_eq!(
                self.init_state.get(),
                InitState::ObjectConstructing,
                "Base<T> is already initialized, or holds a script instance"
            );

            self.init_state.set(InitState::ObjectInitialized);
        }

        // May return whether there is a "surplus" strong ref in the future, as self.extra_strong_ref.borrow().is_some().
    }

    /// Returns a [`Gd`] referencing the base object, assuming the derived object is fully constructed.
    #[doc(hidden)]
    pub fn __fully_constructed_gd(&self) -> Gd<T> {
        #[cfg(debug_assertions)] // debug_assert! still checks existence of symbols.
        assert!(
            !self.is_initializing(),
            "WithBaseField::to_gd(), base(), base_mut() can only be called on fully-constructed objects, after I*::init() or Gd::from_init_fn()"
        );

        (*self.obj).clone()
    }

    /// Returns a [`Gd`] referencing the base object, for use in script contexts only.
    #[doc(hidden)]
    pub fn __script_gd(&self) -> Gd<T> {
        // Used internally by `SiMut::base()` and `SiMut::base_mut()` for script re-entrancy.
        // Could maybe add debug validation to ensure script context in the future.
        (*self.obj).clone()
    }

    // Currently only used in outbound virtual calls (for scripts); search for: base_field(self).obj_sys().
    #[doc(hidden)]
    pub fn obj_sys(&self) -> sys::GDExtensionObjectPtr {
        self.obj.obj_sys()
    }

    // Internal use only, do not make public.
    #[cfg(feature = "debug-log")]
    pub(crate) fn debug_instance_id(&self) -> crate::obj::InstanceId {
        self.obj.instance_id()
    }

    /// Returns a [`Gd`] referencing the base object, for use in script contexts only.
    pub(crate) fn to_script_gd(&self) -> Gd<T> {
        #[cfg(debug_assertions)]
        assert_eq!(
            self.init_state.get(),
            InitState::Script,
            "to_script_gd() can only be called on script-context Base objects"
        );

        (*self.obj).clone()
    }

    /// Returns `true` if this `Base<T>` is currently in the initializing state.
    #[cfg(debug_assertions)]
    fn is_initializing(&self) -> bool {
        self.init_state.get() == InitState::ObjectConstructing
    }

    /// Returns a [`Gd`] referencing the base object, assuming the derived object is fully constructed.
    #[doc(hidden)]
    pub fn __constructed_gd(&self) -> Gd<T> {
        #[cfg(debug_assertions)] // debug_assert! still checks existence of symbols.
        assert!(
            !self.is_initializing(),
            "WithBaseField::to_gd(), base(), base_mut() can only be called on fully-constructed objects, after I*::init() or Gd::from_init_fn()"
        );

        (*self.obj).clone()
    }
}

impl<T: GodotClass> Debug for Base<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        classes::debug_string(&self.obj, f, "Base")
    }
}

impl<T: GodotClass> Display for Base<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        classes::display_string(&self.obj, f)
    }
}
