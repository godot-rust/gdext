/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::hash_map::Entry as HashEntry;

use crate::builtin::Callable;
use crate::obj::base_init::InitState;
use crate::obj::{Base, Gd, GodotClass, InstanceId};
use crate::{classes, sys};

thread_local! {
    /// Extra strong references for each instance ID, needed for [`Base::to_init_gd()`].
    ///
    /// At the moment, all Godot objects must be accessed from the main thread, because their deferred destruction (`Drop`) runs on the
    /// main thread, too. This may be relaxed in the future, and a `sys::Global` could be used instead of a `thread_local!`.
    static PENDING_STRONG_REFS: RefCell<HashMap<InstanceId, Gd<classes::RefCounted>>> = RefCell::new(HashMap::new());
}

impl<T: GodotClass> Base<T> {
    #[doc(hidden)]
    pub(crate) fn to_init_gd_inner(&self) -> Gd<T> {
        sys::balanced_assert!(
            self.init_state.is_initializing(),
            "Base::to_init_gd() can only be called during object initialization, inside I*::init() or Gd::from_init_fn()"
        );

        // For manually-managed objects, regular clone is fine.
        // Only static type matters, because this happens immediately after initialization, so T is both static and dynamic type.
        if !<T::Memory as crate::obj::bounds::Memory>::IS_REF_COUNTED {
            return Gd::clone(&self.obj);
        }

        sys::balanced_assert!(
            sys::is_main_thread(),
            "Base::to_init_gd() can only be called on the main thread for ref-counted objects (for now)"
        );

        // First time handing out a Gd<T>, we need to take measures to temporarily upgrade the Base's weak pointer to a strong one.
        // During the initialization phase (derived object being constructed), increment refcount by 1.
        let instance_id = self.obj.instance_id();
        let mut defer_unref = false;
        PENDING_STRONG_REFS.with(|refs| {
            let mut pending_refs = refs.borrow_mut();
            if let HashEntry::Vacant(e) = pending_refs.entry(instance_id) {
                let strong_ref: Gd<T> = unsafe { Gd::from_obj_sys(self.obj.obj_sys()) };

                // T: Inherits<RefCounted> is confirmed by IS_REF_COUNTED check above.
                // Transfer ownership via owned_cast (no refcount change): original Gd<T> is consumed.
                let raw = strong_ref
                    .raw
                    .owned_cast::<classes::RefCounted>()
                    .expect("IS_REF_COUNTED guarantees T inherits RefCounted");

                e.insert(Gd { raw });
                defer_unref = true;
            }
        });

        // Only defer the drop-strong-ref function if we actually inserted the pending strong-ref.
        // Avoids "ERROR: Error calling deferred method: '':"
        if defer_unref {
            let name = format!("Base<{}> deferred unref", T::class_id());
            let callable = Callable::from_once_fn(name, move |_args| {
                Self::drop_strong_ref(instance_id);
            });

            // Use Callable::call_deferred() instead of Gd::apply_deferred(). The latter implicitly borrows &mut self,
            // causing a "destroyed while bind was active" panic.
            callable.call_deferred(&[]);
        }

        (*self.obj).clone()
    }

    /// Drops any extra strong references, possibly causing object destruction.
    fn drop_strong_ref(instance_id: InstanceId) {
        PENDING_STRONG_REFS.with(|refs| {
            let mut pending_refs = refs.borrow_mut();
            let strong_ref = pending_refs.remove(&instance_id);
            sys::strict_assert!(
                strong_ref.is_some(),
                "Base unexpectedly had its strong ref rug-pulled"
            );

            // Editor creates instances of given class for various purposes (getting class docs, default values...)
            // and frees them instantly before our callable can be executed.
            // Perform "weak" drop instead of "strong" one iff our instance is no longer valid.
            if !instance_id.lookup_validity() {
                strong_ref.unwrap().drop_weak();
            }

            // Triggers RawGd::drop() -> dec-ref -> possibly object destruction.
        });
    }

    /// Finalizes the initialization of this `Base<T>`.
    pub(crate) fn mark_initialized(&mut self) {
        self.init_state.mark_initialized();
    }
}

pub use implementation::*;

#[cfg(safeguards_balanced)]
mod implementation {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;

    /// Tracks the initialization state of this `Base<T>`.
    ///
    /// Rc allows to "copy-construct" the base from an existing one, while still affecting the user-instance through the original `Base<T>`.
    #[derive(Clone, Debug)]
    pub struct InitTracker {
        init_state: Rc<Cell<InitState>>,
    }

    impl InitTracker {
        pub fn new(state: InitState) -> Self {
            Self {
                init_state: Rc::new(Cell::new(state)),
            }
        }

        pub fn is_initializing(&self) -> bool {
            self.init_state.get() == InitState::ObjectConstructing
        }

        /// Finalizes the initialization of this `Base<T>`.
        pub fn mark_initialized(&self) {
            assert_eq!(
                self.init_state.get(),
                InitState::ObjectConstructing,
                "Base<T> is already initialized, or holds a script instance"
            );
            self.init_state.set(InitState::ObjectInitialized);
        }

        // Asserts live here so call sites don't need cfg.
        pub fn assert_constructed(&self) {
            sys::balanced_assert!(
                !self.is_initializing(),
                "WithBaseField::base(), base_mut(), to_gd() can only be called on \
                  fully-constructed objects, after I*::init() or Gd::from_init_fn()"
            );
        }

        pub fn assert_script(&self) {
            sys::balanced_assert!(
                self.init_state.get() == InitState::Script,
                "to_script_passive() can only be called on script-context Base objects"
            );
        }
    }
}

#[cfg(not(safeguards_balanced))]
mod implementation {
    use super::InitState;

    /// Tracks the initialization state of this `Base<T>`.
    ///
    /// Rc allows to "copy-construct" the base from an existing one, while still affecting the user-instance through the original `Base<T>`.
    #[derive(Clone)]
    pub struct InitTracker;

    impl InitTracker {
        pub fn new(_state: InitState) -> Self {
            Self
        }
        pub fn is_initializing(&self) -> bool {
            false
        }
        pub fn mark_initialized(&self) {}
        pub fn assert_constructed(&self) {}
        pub fn assert_script(&self) {}
    }
}
