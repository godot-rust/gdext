/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::{fmt, ptr};

use godot_ffi as sys;
use sys::{ffi_methods, ExtVariantType, GodotFfi};

use crate::builtin::{inner, Array, Callable, StringName, VarDictionary, Variant};
use crate::classes::object::ConnectFlags;
use crate::classes::Object;
use crate::global::Error;
use crate::meta;
use crate::meta::{FromGodot, GodotType, ToGodot};
use crate::obj::bounds::DynMemory;
use crate::obj::{Bounds, EngineBitfield, Gd, GodotClass, InstanceId};

/// Untyped Godot signal.
///
/// Signals are composed of a pointer to an `Object` and the name of the signal on this object.
///
/// In Rust, you might want to work with type-safe signals, available under the [`TypedSignal`](crate::registry::signal::TypedSignal) struct.
///
/// # Godot docs
/// [`Signal` (stable)](https://docs.godotengine.org/en/stable/classes/class_signal.html)
pub struct Signal {
    opaque: sys::types::OpaqueSignal,
}

impl Signal {
    fn from_opaque(opaque: sys::types::OpaqueSignal) -> Self {
        Self { opaque }
    }

    /// Create a signal for the signal `object::signal_name`.
    ///
    /// _Godot equivalent: `Signal(Object object, StringName signal)`_
    pub fn from_object_signal<T, S>(object: &Gd<T>, signal_name: S) -> Self
    where
        T: GodotClass,
        S: meta::AsArg<StringName>,
    {
        meta::arg_into_ref!(signal_name);

        unsafe {
            Self::new_with_uninit(|self_ptr| {
                let ctor = sys::builtin_fn!(signal_from_object_signal);
                let raw = object.to_ffi();
                let args = [raw.as_arg_ptr(), signal_name.sys()];
                ctor(self_ptr, args.as_ptr());
            })
        }
    }

    /// Creates an invalid/empty signal that cannot be called.
    ///
    /// _Godot equivalent: `Signal()`_
    pub fn invalid() -> Self {
        unsafe {
            Self::new_with_uninit(|self_ptr| {
                let ctor = sys::builtin_fn!(signal_construct_default);
                ctor(self_ptr, ptr::null_mut())
            })
        }
    }

    /// Connect signal to a callable.
    ///
    /// To provide flags, see [`connect_flags()`][Self::connect_flags].
    pub fn connect(&self, callable: &Callable) -> Error {
        let error = self.as_inner().connect(callable, 0i64);

        Error::from_godot(error as i32)
    }

    /// Connect signal to a callable, customizing with flags.
    ///
    /// Optional flags can be also added to configure the connection's behavior (see [`ConnectFlags`](ConnectFlags) constants).
    /// You can provide additional arguments to the connected callable by using `Callable::bind`.
    ///
    /// A signal can only be connected once to the same [`Callable`]. If the signal is already connected, returns [`Error::ERR_INVALID_PARAMETER`]
    /// and pushes an error message, unless the signal is connected with [`ConnectFlags::REFERENCE_COUNTED`](ConnectFlags::REFERENCE_COUNTED).
    /// To prevent this, check for existing connections with [`is_connected()`][Self::is_connected].
    pub fn connect_flags(&self, callable: &Callable, flags: ConnectFlags) -> Error {
        let error = self.as_inner().connect(callable, flags.ord() as i64);

        Error::from_godot(error as i32)
    }

    /// Disconnects this signal from the specified [`Callable`].
    ///
    /// If the connection does not exist, generates an error. Use [`is_connected()`](Self::is_connected) to make sure that the connection exists.
    pub fn disconnect(&self, callable: &Callable) {
        self.as_inner().disconnect(callable);
    }

    /// Emits this signal.
    ///
    /// All Callables connected to this signal will be triggered.
    pub fn emit(&self, varargs: &[Variant]) {
        let Some(mut object) = self.object() else {
            return;
        };

        object.emit_signal(&self.name(), varargs);
    }

    /// Returns an [`Array`] of connections for this signal.
    ///
    /// Each connection is represented as a Dictionary that contains three entries:
    ///  - `signal` is a reference to this [`Signal`];
    ///  - `callable` is a reference to the connected [`Callable`];
    ///  - `flags` is a combination of [`ConnectFlags`](ConnectFlags).
    ///
    /// _Godot equivalent: `get_connections`_
    pub fn connections(&self) -> Array<VarDictionary> {
        self.as_inner()
            .get_connections()
            .iter_shared()
            .map(|variant| variant.to())
            .collect()
    }

    /// Returns the name of the signal.
    pub fn name(&self) -> StringName {
        self.as_inner().get_name()
    }

    /// Returns the object to which this signal belongs.
    ///
    /// Returns [`None`] when this signal doesn't have any object, or the object is dead. You can differentiate these two situations using
    /// [`object_id()`][Self::object_id].
    ///
    /// _Godot equivalent: `get_object`_
    pub fn object(&self) -> Option<Gd<Object>> {
        self.as_inner().get_object().map(|mut object| {
            <Object as Bounds>::DynMemory::maybe_inc_ref(&mut object.raw);
            object
        })
    }

    /// Returns the ID of this signal's object, see also [`Gd::instance_id`].
    ///
    /// Returns [`None`] when this signal doesn't have any object.
    ///
    /// If the pointed-to object is dead, the ID will still be returned. Use [`object()`][Self::object] to check for liveness.
    ///
    /// _Godot equivalent: `get_object_id`_
    pub fn object_id(&self) -> Option<InstanceId> {
        let id = self.as_inner().get_object_id();
        InstanceId::try_from_i64(id)
    }

    /// Returns `true` if the specified [`Callable`] is connected to this signal.
    pub fn is_connected(&self, callable: &Callable) -> bool {
        self.as_inner().is_connected(callable)
    }

    /// Returns `true` if the signal's name does not exist in its object, or the object is not valid.
    pub fn is_null(&self) -> bool {
        self.as_inner().is_null()
    }

    #[doc(hidden)]
    pub fn as_inner(&self) -> inner::InnerSignal<'_> {
        inner::InnerSignal::from_outer(self)
    }
}

// SAFETY:
// The `opaque` in `Signal` is just a pair of pointers, and requires no special initialization or cleanup
// beyond what is done in `from_opaque` and `drop`. So using `*mut Opaque` is safe.
unsafe impl GodotFfi for Signal {
    const VARIANT_TYPE: ExtVariantType = ExtVariantType::Concrete(sys::VariantType::SIGNAL);

    ffi_methods! { type sys::GDExtensionTypePtr = *mut Opaque;
        fn new_from_sys;
        fn new_with_uninit;
        fn from_arg_ptr;
        fn sys;
        fn sys_mut;
        fn move_return_ptr;
    }

    unsafe fn new_with_init(init_fn: impl FnOnce(sys::GDExtensionTypePtr)) -> Self {
        let mut result = Self::invalid();
        init_fn(result.sys_mut());
        result
    }
}

impl_builtin_traits! {
    for Signal {
        Clone => signal_construct_copy;
        Drop => signal_destroy;
        PartialEq => signal_operator_equal;
    }
}

meta::impl_godot_as_self!(Signal: ByRef);

impl fmt::Debug for Signal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.name();
        let object = self.object();

        f.debug_struct("signal")
            .field("name", &name)
            .field("object", &object)
            .finish()
    }
}

impl fmt::Display for Signal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_variant())
    }
}
