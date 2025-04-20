/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Implementation note:
// If this codes too much code bloat / compilation time due to excessive monomorphization of C, it's possible to type-erase this, as
// the internal representation just needs Object. This would allow that all signal collections look the same. It would however make
//

use crate::classes::Object;
use crate::obj::{Gd, GodotClass, WithBaseField, WithSignals, WithUserSignals};

/// Indirection from [`TypedSignal`] to the actual Godot object.
#[doc(hidden)]
pub trait SignalObject<'c> {
    fn with_object_mut(&mut self, f: impl FnOnce(&mut Object));
    fn to_owned_object(&self) -> Gd<Object>;
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Impl for signals() on user classes.

/// Links to a Godot object, either via reference (for `&mut self` uses) or via `Gd`.
///
/// Needs to differentiate the two cases:
/// - `C` is a user object implementing `WithBaseField`, possibly having access from within the class.
/// - `C` is an engine object, so only accessible through `Gd<C>`.
#[doc(hidden)]
pub enum UserSignalObject<'c, C> {
    /// Helpful for emit: reuse `&mut self` from within the `impl` block, goes through `base_mut()` re-borrowing and thus allows re-entrant calls
    /// through Godot.
    Internal { self_mut: &'c mut C },
    //Internal { obj_mut: &'c mut classes::Object },
    /// From outside, based on `Gd` pointer.
    External { gd: Gd<Object> },
}

impl<'c, C> UserSignalObject<'c, C>
where
    // 2nd bound necessary, so generics match for TypedSignal construction.
    C: WithUserSignals + WithSignals<__SignalObj<'c> = UserSignalObject<'c, C>>,
{
    #[inline]
    pub fn from_external(object: Gd<C>) -> Self {
        Self::External {
            gd: object.upcast(),
        }
    }

    #[inline]
    pub fn from_internal(self_mut: &'c mut C) -> Self {
        Self::Internal { self_mut }
    }
}

impl<'c, C: WithUserSignals> SignalObject<'c> for UserSignalObject<'c, C> {
    #[inline]
    fn with_object_mut(&mut self, f: impl FnOnce(&mut Object)) {
        match self {
            Self::Internal { self_mut } => {
                let mut guard = <C as WithBaseField>::base_mut(*self_mut);
                f(guard.upcast_object_mut())
            }
            Self::External { gd } => f(gd.upcast_object_mut()),
        }
    }

    #[inline]
    fn to_owned_object(&self) -> Gd<Object> {
        match self {
            // SignalObject::Internal { obj_mut } => crate::private::rebuild_gd(*obj_mut),
            Self::Internal { self_mut } => <C as WithBaseField>::to_gd(self_mut).upcast_object(),
            Self::External { gd } => gd.clone(),
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Impl for signals() on engine classes.

impl<C: WithSignals> SignalObject<'_> for Gd<C> {
    #[inline]
    fn with_object_mut(&mut self, f: impl FnOnce(&mut Object)) {
        f(self.upcast_object_mut())
    }

    #[inline]
    fn to_owned_object(&self) -> Gd<Object> {
        self.clone().upcast_object()
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Helpers for SignalCollection upcasts.

pub fn signal_collection_to_base<'r, 'c, C, Derived>(
    derived: &'r Derived::SignalCollection<'c, C>,
) -> &'r <<Derived as GodotClass>::Base as WithSignals>::SignalCollection<'c, C>
where
    C: WithSignals,
    Derived: WithSignals<Base: WithSignals>,
{
    type BaseCollection<'c, C, Derived> =
        <<Derived as GodotClass>::Base as WithSignals>::SignalCollection<'c, C>;

    let derived_collection_ptr = std::ptr::from_ref(derived);
    let base_collection_ptr = derived_collection_ptr.cast::<BaseCollection<'c, C, Derived>>();

    // SAFETY:
    // - Signal collections have the same memory layout, independent of their enclosing class. (While they may differ depending on
    //   internal/external usage, upcasts
    // - The `Inherits` bound additionally ensures that all signals present in Base are also present in Derived, i.e.
    //   reducing the collection to a smaller subset of signals is safe.
    // - The lifetimes remain unchanged.
    unsafe { &*base_collection_ptr }
}

pub fn signal_collection_to_base_mut<'r, 'c, C, Derived>(
    derived: &'r mut Derived::SignalCollection<'c, C>,
) -> &'r mut <<Derived as GodotClass>::Base as WithSignals>::SignalCollection<'c, C>
where
    C: WithSignals,
    Derived: WithSignals<Base: WithSignals>,
{
    type BaseCollection<'c, C, Derived> =
        <<Derived as GodotClass>::Base as WithSignals>::SignalCollection<'c, C>;

    let derived_collection_ptr = std::ptr::from_mut(derived);
    let base_collection_ptr = derived_collection_ptr.cast::<BaseCollection<'c, C, Derived>>();

    // SAFETY: see signal_collection_to_base().
    unsafe { &mut *base_collection_ptr }
}

/* Currently unused, but kept around as it's not unlikely we need some form of this.

fn upcast_signal_collection<'r, 'c, C, Derived, Base>(
    derived: &'r Derived::SignalCollection<'c, C>,
) -> &'r Base::SignalCollection<'c, C>
where
    C: WithSignals,
    Derived: WithSignals, // + Inherits<Base>,
    Base: WithSignals,
{
    let derived_collection_ptr = std::ptr::from_ref(derived);
    let base_collection_ptr = derived_collection_ptr.cast::<Base::SignalCollection<'c, C>>();

    // SAFETY: see signal_collection_to_base().
    unsafe { &*base_collection_ptr }
}

fn upcast_signal_collection_mut<'r, 'c, C, Derived, Base>(
    derived: &'r mut Derived::SignalCollection<'c, C>,
) -> &'r mut Base::SignalCollection<'c, C>
where
    C: WithSignals,
    Derived: WithSignals + Inherits<Base>,
    Base: WithSignals,
{
    let derived_collection_ptr = std::ptr::from_mut(derived);
    let base_collection_ptr = derived_collection_ptr.cast::<Base::SignalCollection<'c, C>>();

    // SAFETY: see signal_collection_to_base().
    unsafe { &mut *base_collection_ptr }
}
*/
