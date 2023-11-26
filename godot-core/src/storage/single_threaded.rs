use std::any::type_name;
use std::cell;

use crate::obj::{Base, GodotClass};
use crate::out;

use super::Lifecycle;

/// Manages storage and lifecycle of user's extension class instances.
pub struct InstanceStorage<T: GodotClass> {
    user_instance: cell::RefCell<T>,
    pub(super) base: Base<T::Base>,

    // Declared after `user_instance`, is dropped last
    pub(super) lifecycle: cell::Cell<Lifecycle>,
    godot_ref_count: cell::Cell<u32>,
}

impl<T: GodotClass> super::Storage for InstanceStorage<T> {
    type Instance = T;

    type RefGuard<'a> = cell::Ref<'a, T>;

    type MutGuard<'a> = cell::RefMut<'a, T>;

    fn construct(
        user_instance: Self::Instance,
        base: Base<<Self::Instance as GodotClass>::Base>,
    ) -> Self {
        out!("    Storage::construct             <{}>", type_name::<T>());

        Self {
            user_instance: cell::RefCell::new(user_instance),
            base,
            lifecycle: cell::Cell::new(Lifecycle::Alive),
            godot_ref_count: cell::Cell::new(1),
        }
    }

    fn is_bound(&self) -> bool {
        // Needs to borrow mutably, otherwise it succeeds if shared borrows are alive.
        self.user_instance.try_borrow_mut().is_err()
    }

    fn base(&self) -> &Base<<Self::Instance as GodotClass>::Base> {
        &self.base
    }

    fn get(&self) -> Self::RefGuard<'_> {
        self.user_instance.try_borrow().unwrap_or_else(|_e| {
            panic!(
                "Gd<T>::bind() failed, already bound; T = {}.\n  \
                     Make sure there is no &mut T live at the time.\n  \
                     This often occurs when calling a GDScript function/signal from Rust, which then calls again Rust code.",
                type_name::<T>()
            )
        })
    }

    fn get_mut(&self) -> Self::MutGuard<'_> {
        self.user_instance.try_borrow_mut().unwrap_or_else(|_e| {
            panic!(
                "Gd<T>::bind_mut() failed, already bound; T = {}.\n  \
                     Make sure there is no &T or &mut T live at the time.\n  \
                     This often occurs when calling a GDScript function/signal from Rust, which then calls again Rust code.",
                type_name::<T>()
            )
        })
    }

    fn get_lifecycle(&self) -> Lifecycle {
        self.lifecycle.get()
    }

    fn set_lifecycle(&self, lifecycle: Lifecycle) {
        self.lifecycle.set(lifecycle)
    }
}
impl<T: GodotClass> super::StorageRefCounted for InstanceStorage<T> {
    fn godot_ref_count(&self) -> u32 {
        self.godot_ref_count.get()
    }

    fn on_inc_ref(&self) {
        let refc = self.godot_ref_count.get() + 1;
        self.godot_ref_count.set(refc);

        out!(
            "    Storage::on_inc_ref (rc={})     <{}>", // -- {:?}",
            refc,
            type_name::<T>(),
            //self.user_instance
        );
    }

    fn on_dec_ref(&self) {
        let refc = self.godot_ref_count.get() - 1;
        self.godot_ref_count.set(refc);

        out!(
            "  | Storage::on_dec_ref (rc={})     <{}>", // -- {:?}",
            refc,
            type_name::<T>(),
            //self.user_instance
        );
    }
}
