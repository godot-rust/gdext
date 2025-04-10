/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::obj::{BaseMut, Gd, GodotClass, Inherits, WithSignals, WithUserSignals};
use std::marker::PhantomData;
use crate::classes::Object;

/// Indirection from [`TypedSignal`] to the actual Godot object.
///
/// Links to a Godot object, either via reference (for `&mut self` uses) or via `Gd`.
///
/// Needs to differentiate the two cases:
/// - `C` is a user object implementing `WithBaseField`, possibly having access from within the class.
/// - `C` is an engine object, so only accessible through `Gd<C>`.
#[doc(hidden)]
pub enum SignalObject<'c> {
    /// Helpful for emit: reuse `&mut self` from within the `impl` block, goes through `base_mut()` re-borrowing and thus allows re-entrant calls
    /// through Godot.
    Internal { base_mut: Box<dyn ErasedBase<'c>> },
    //Internal { obj_mut: &'c mut classes::Object },
    /// From outside, based on `Gd` pointer.
    External { gd: Gd<Object> },
}

impl SignalObject<'_> {
    pub fn with_object_mut(&mut self, f: impl FnOnce(&mut Object)) {
        match self {
            SignalObject::Internal { base_mut } => f(base_mut.borrow_object_mut()),
            SignalObject::External { gd } => f(gd.upcast_object_mut()),
        }
    }

   pub  fn to_owned_object(&self) -> Gd<Object> {
        match self {
            // SignalObject::Internal { obj_mut } => crate::private::rebuild_gd(*obj_mut),
            SignalObject::Internal { base_mut } => base_mut.to_owned_object(),
            SignalObject::External { gd } => gd.clone(),
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Wrapper around a signal object that pretends to be generic.
#[doc(hidden)]
pub struct SignalColl<'c, C> {
    object: SignalObject<'c>,
    phantom: PhantomData<C>,
}

impl<'c, C> SignalColl<'c, C>
where
    C: WithSignals,
{
    pub fn from_external(object: Gd<C>) -> Self {
        Self {
            object: SignalObject::External { gd: object.upcast() },
            phantom: PhantomData,
        }
    }

    pub fn from_internal(self_mut: &'c mut C) -> Self
    where
        C: WithUserSignals,
    {
        // Potential optimization: we could store the raw object pointer, however the BaseMut<T> guard would need to stay alive,
        // to ensure the reborrowing works and emit() can reuse &mut self without running into double-borrow on the handler function.
        // So there would still need to be a way to *store* (not access) BaseMut<T> in an erased way, possibly with intrusive changes.
        let base_mut = Box::new(self_mut.base_mut());

        Self {
            object: SignalObject::Internal { base_mut },
            phantom: PhantomData,
        }
    }
}

trait ErasedBase<'c> {
    fn to_owned_object(&self) -> Gd<Object>;
    fn borrow_object_mut(&mut self) -> &mut Object;
}

/// Type-erases a `BaseMut` exclusive guard from its `T` parameter.
///
/// Necessary to ensure that all signal collections have the same internal memory layout, and `Deref`/`DerefMut` to "super"
/// collections is possible. `BaseMut` contains `Gd`, and `Gd<UserClass>`/`Gd<EngineClass>` have different memory layouts.
///
/// See possible optimization note at use site.
impl<'c, T> ErasedBase<'c> for BaseMut<'c, T>
where
    T: Inherits<Object>
{
    fn to_owned_object(&self) -> Gd<Object> {
        let deref =  &**self;
        deref.clone().upcast()
    }

    fn borrow_object_mut(&mut self) -> &mut Object {
        self.upcast_object_mut()
    }
}
