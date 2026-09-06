/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::ffi::c_void;

use godot_ffi as sys;
use sys::interface_fn;

use crate::builtin::{StringName, Variant};
use crate::meta::private_reexport::{CallContext, Signature};
use crate::meta::{ClassId, EngineToGodot, GodotConvert, InParamTuple, sig_params};
use crate::registry::info::{MethodFlags, PropertyInfo};

/// Info relating to an argument or return type in a method.
pub struct MethodParamOrReturnInfo {
    pub(crate) info: PropertyInfo,
    metadata: sys::GDExtensionClassMethodArgumentMetadata,
}

impl MethodParamOrReturnInfo {
    pub fn new(info: PropertyInfo, metadata: sys::GDExtensionClassMethodArgumentMetadata) -> Self {
        Self { info, metadata }
    }

    /// Creates parameter info for type `T`.
    pub fn for_parameter<T: GodotConvert>(param_name: &str) -> Self {
        let shape = T::godot_shape();
        Self {
            info: shape.to_method_signature_property(param_name),
            metadata: shape.param_metadata().to_sys(),
        }
    }

    /// Creates return type info for type `T`.
    pub fn for_return<T: GodotConvert>() -> Option<Self> {
        let shape = T::godot_shape();
        Some(Self {
            info: shape.to_method_signature_property(""),
            metadata: shape.param_metadata().to_sys(),
        })
    }
}

/// All info needed to register a method for a class with Godot.
pub struct ClassMethodInfo {
    class_id: ClassId,
    method_name: StringName,
    call_func: sys::GDExtensionClassMethodCall,
    ptrcall_func: sys::GDExtensionClassMethodPtrCall,
    method_userdata: *mut c_void,
    method_flags: MethodFlags,
    return_value: Option<MethodParamOrReturnInfo>,
    arguments: Vec<MethodParamOrReturnInfo>,
    /// Whether default arguments are real "arguments" is controversial. From the function PoV they are, but for the caller,
    /// they are just pre-set values to fill in for missing arguments.
    ///
    /// Points into the [`MethodUserdata`] stored for this class, which outlives the registration.
    default_arguments: Vec<sys::GDExtensionVariantPtr>,
}

impl ClassMethodInfo {
    /// Builds the method info from `method_data`, whose allocation is owned by the class's registry entry and passed to Godot as `method_userdata`.
    ///
    /// # Safety
    /// `method_data`'s function must interpret its instance pointer as an instance of `class_id`, and `method_flags` must match
    /// the receiver (e.g. [`MethodFlags::STATIC`] only for functions ignoring the instance pointer).
    pub unsafe fn from_signature<Params, Ret>(
        class_id: ClassId,
        method_name: StringName,
        method_flags: MethodFlags,
        param_names: &[&str],
        method_data: MethodUserdata<Params, Ret>,
    ) -> Self
    where
        Params: InParamTuple + 'static,
        Ret: EngineToGodot + 'static,
    {
        let method_userdata = method_data.into_raw();

        // SAFETY: method_userdata comes from MethodUserdata::<Params, Ret>::into_raw(), so it matches VTABLE and is not aliased.
        unsafe {
            Self::from_erased(
                class_id,
                method_name,
                method_flags,
                param_names,
                MethodUserdata::<Params, Ret>::VTABLE,
                method_userdata,
            )
        }
    }

    /// Non-generic part of [`Self::from_signature()`], reaching `Params`/`Ret` only through `vtable`.
    ///
    /// # Safety
    /// `method_userdata` must come from [`MethodUserdata::into_raw()`] for the `Params`/`Ret` matching `vtable`, and must not be aliased.
    unsafe fn from_erased(
        class_id: ClassId,
        method_name: StringName,
        method_flags: MethodFlags,
        param_names: &[&str],
        vtable: &'static MethodVTable,
        method_userdata: *mut c_void,
    ) -> Self {
        use crate::obj::EngineBitfield as _;

        let return_value = (vtable.return_info_fn)();
        let arguments = (vtable.param_info_fn)(param_names);

        // SAFETY: guaranteed by the caller.
        let header = unsafe { MethodHeader::from_userdata_ptr(method_userdata) };
        let defaults = &header.default_arguments;

        assert!(
            defaults.len() <= arguments.len(),
            "cannot have more default arguments than parameters"
        );

        // Virtual methods are registered through `classdb_register_extension_class_virtual_method()`, which takes neither callbacks nor
        // userdata, nor default arguments -- so we don't keep anything for those.
        let mut call_func = None;
        let mut ptrcall_func = None;
        let mut userdata = std::ptr::null_mut();
        let mut default_arguments = Vec::new();

        if method_flags.is_set(MethodFlags::VIRTUAL) {
            // SAFETY: guaranteed by the caller; the pointer is dropped exactly once, here.
            unsafe { (vtable.drop_fn)(method_userdata) };
        } else {
            call_func = vtable.call_func;
            ptrcall_func = vtable.ptrcall_func;
            userdata = method_userdata;

            // default_arguments points into the Vec owned by the userdata, which store_method_userdata() keeps alive.
            default_arguments = default_argument_ptrs(defaults);

            let erased = ErasedMethodUserdata {
                ptr: method_userdata,
                drop_fn: vtable.drop_fn,
            };
            crate::registry::class::store_method_userdata(class_id, erased);
        }

        Self {
            class_id,
            method_name,
            call_func,
            ptrcall_func,
            method_userdata: userdata,
            method_flags,
            return_value,
            arguments,
            default_arguments,
        }
    }

    pub fn register_extension_class_method(&self) {
        use crate::obj::EngineBitfield as _;

        let (return_value_info, return_value_metadata) = match &self.return_value {
            Some(info) => (Some(&info.info), info.metadata),
            None => (None, 0),
        };

        let mut return_value_sys = return_value_info
            .as_ref()
            .map(|info| info.property_sys())
            .unwrap_or(PropertyInfo::empty_sys());

        let mut arguments_info_sys: Vec<sys::GDExtensionPropertyInfo> = self
            .arguments
            .iter()
            .map(|argument| argument.info.property_sys())
            .collect();

        let mut arguments_metadata: Vec<sys::GDExtensionClassMethodArgumentMetadata> =
            self.arguments.iter().map(|info| info.metadata).collect();

        let method_info_sys = sys::GDExtensionClassMethodInfo {
            name: sys::SysPtr::force_mut(self.method_name.string_sys()),
            method_userdata: self.method_userdata,
            call_func: self.call_func,
            ptrcall_func: self.ptrcall_func,
            method_flags: self.method_flags.ord() as u32,
            has_return_value: self.return_value.is_some() as u8,
            return_value_info: std::ptr::addr_of_mut!(return_value_sys),
            return_value_metadata,
            argument_count: self.argument_count(),
            arguments_info: arguments_info_sys.as_mut_ptr(),
            arguments_metadata: arguments_metadata.as_mut_ptr(),
            default_argument_count: self.default_argument_count(),
            // Godot copies the default arguments during registration -- the array itself is only read.
            default_arguments: self.default_arguments.as_ptr().cast_mut(),
        };

        if self.method_flags.is_set(MethodFlags::VIRTUAL) {
            self.register_virtual_class_method(method_info_sys, return_value_sys);
        } else {
            self.register_nonvirtual_class_method(method_info_sys);
        }
    }

    fn register_nonvirtual_class_method(&self, method_info_sys: sys::GDExtensionClassMethodInfo) {
        // Only for non-virtual methods. Godot keeps virtual methods in a separate map, which isn't exposed through ClassDB.
        crate::registry::reg_validation::validate_method(self.class_id, &self.method_name);

        // SAFETY: The lifetime of the data we use here is at least as long as this function's scope. So we can
        // safely call this function without issue.
        //
        // Null pointers will only be passed along if we indicate to Godot that they are unused.
        unsafe {
            interface_fn!(classdb_register_extension_class_method)(
                sys::get_library(),
                self.class_id.string_sys(),
                std::ptr::addr_of!(method_info_sys),
            )
        }
    }

    #[cfg(since_api = "4.3")]
    fn register_virtual_class_method(
        &self,
        normal_method_info: sys::GDExtensionClassMethodInfo,
        return_value_sys: sys::GDExtensionPropertyInfo, // passed separately because value, not pointer.
    ) {
        // Copy everything possible from regular method info.
        let method_info_sys = sys::GDExtensionClassVirtualMethodInfo {
            name: normal_method_info.name,
            method_flags: normal_method_info.method_flags,
            return_value: return_value_sys,
            return_value_metadata: normal_method_info.return_value_metadata,
            argument_count: normal_method_info.argument_count,
            arguments: normal_method_info.arguments_info,
            arguments_metadata: normal_method_info.arguments_metadata,
        };

        // SAFETY: Godot only needs arguments to be alive during the method call.
        unsafe {
            interface_fn!(classdb_register_extension_class_virtual_method)(
                sys::get_library(),
                self.class_id.string_sys(),
                std::ptr::addr_of!(method_info_sys),
            )
        }
    }

    // Polyfill doing nothing.
    #[cfg(before_api = "4.3")]
    fn register_virtual_class_method(
        &self,
        _normal_method_info: sys::GDExtensionClassMethodInfo,
        _return_value_sys: sys::GDExtensionPropertyInfo,
    ) {
    }

    fn argument_count(&self) -> u32 {
        self.arguments
            .len()
            .try_into()
            .expect("arguments length should fit in u32")
    }

    fn default_argument_count(&self) -> u32 {
        self.default_arguments
            .len()
            .try_into()
            .expect("arguments length should fit in u32")
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Shared #[func] callbacks

/// Per-signature values needed by [`ClassMethodInfo::from_erased()`], so it stays non-generic.
struct MethodVTable {
    call_func: sys::GDExtensionClassMethodCall,
    ptrcall_func: sys::GDExtensionClassMethodPtrCall,
    drop_fn: unsafe fn(*mut c_void),
    param_info_fn: fn(&[&str]) -> Vec<MethodParamOrReturnInfo>,
    return_info_fn: fn() -> Option<MethodParamOrReturnInfo>,
}

/// Everything the FFI callbacks need to invoke one `#[func]`, passed to Godot as its `method_userdata`.
///
/// Erasure for per-method code: `varcall_callback()` and `ptrcall_callback()` are instantiated once per `(Params, Ret)` pair, instead of once
/// per registered function.
#[repr(C)]
pub struct MethodUserdata<Params, Ret> {
    header: MethodHeader,
    func: fn(sys::GDExtensionClassInstancePtr, Params) -> Ret,
}

/// The part of [`MethodUserdata`] that does not mention `Params`/`Ret`.
///
/// `MethodUserdata` is `#[repr(C)]` and starts with this, so code that only has an untyped pointer can still read these fields --
/// no accessor generated per signature.
struct MethodHeader {
    class_name: &'static str,
    method_name: &'static str,
    default_arguments: MethodDefaults,
}

impl MethodHeader {
    /// # Safety
    /// `ptr` must come from [`MethodUserdata::into_raw()`] and point to a live allocation.
    unsafe fn from_userdata_ptr<'a>(ptr: *mut c_void) -> &'a MethodHeader {
        // SAFETY: guaranteed by the caller; `MethodUserdata` is `#[repr(C)]` with this type as its first field, so it lies at offset 0.
        unsafe { &*ptr.cast::<MethodHeader>() }
    }
}

impl<Params, Ret> MethodUserdata<Params, Ret> {
    /// # Safety
    /// `func` must treat its instance pointer as an instance of the class the method is registered for.
    pub unsafe fn new(
        class_name: &'static str,
        method_name: &'static str,
        func: fn(sys::GDExtensionClassInstancePtr, Params) -> Ret,
        default_arguments: Vec<Variant>,
    ) -> Self {
        Self {
            header: MethodHeader {
                class_name,
                method_name,
                default_arguments: MethodDefaults(default_arguments),
            },
            func,
        }
    }

    /// Moves `self` to the heap, returning the pointer passed to Godot as `method_userdata` and reclaimed via [`Self::drop_raw()`].
    fn into_raw(self) -> *mut c_void {
        Box::into_raw(Box::new(self)).cast::<c_void>()
    }

    /// Reconstructs [`ErasedMethodUserdata`] box and drops it. Instantiated once per `(Params, Ret)` pair, not per `#[func]`.
    ///
    /// # Safety
    /// `ptr` must come from [`Self::into_raw()`] and be no longer aliased.
    unsafe fn drop_raw(ptr: *mut c_void) {
        let method_userdata_ptr = ptr.cast::<Self>();

        // SAFETY: guaranteed by the caller.
        drop(unsafe { Box::from_raw(method_userdata_ptr) });
    }
}

impl<Params: InParamTuple, Ret: EngineToGodot> MethodUserdata<Params, Ret> {
    const VTABLE: &'static MethodVTable = &MethodVTable {
        call_func: Some(varcall_callback::<Params, Ret>),
        ptrcall_func: Some(ptrcall_callback::<Params, Ret>),
        drop_fn: Self::drop_raw,
        param_info_fn: sig_params::<Params>,
        return_info_fn: MethodParamOrReturnInfo::for_return::<Ret>,
    };
}

fn default_argument_ptrs(defaults: &[Variant]) -> Vec<sys::GDExtensionVariantPtr> {
    defaults
        .iter()
        .map(|v| sys::SysPtr::force_mut(v.var_sys()))
        .collect()
}

/// Owns one [`MethodUserdata`] without naming its `Params`/`Ret`, so that the class registry can hold methods of all signatures.
///
/// Raw pointer + drop fn instead of `Box<dyn Any>`: in a `cdylib`, a vtable per signature costs ~100kB more stripped than a `fn` pointer,
/// in `.data.rel.ro` and relocations.
pub(crate) struct ErasedMethodUserdata {
    ptr: *mut c_void,
    drop_fn: unsafe fn(*mut c_void),
}

impl Drop for ErasedMethodUserdata {
    fn drop(&mut self) {
        // SAFETY: `ptr` and `drop_fn` match by construction, and run once -- `ErasedMethodUserdata` is neither `Copy` nor cloneable.
        unsafe { (self.drop_fn)(self.ptr) }
    }
}

// SAFETY: the pointee is only read through `&MethodUserdata`, which is `Sync`. Godot registers and unregisters classes on the main thread,
// so the allocation is created and dropped there.
unsafe impl Send for ErasedMethodUserdata {}

/// Default arguments of one `#[func]`, evaluated once at registration and reused by every call.
///
/// [`GodotImmutable`]: crate::meta::GodotImmutable
/// [`opt_default_value()`]: crate::private::opt_default_value
struct MethodDefaults(Vec<Variant>);

// SAFETY: Variants stored inside are never mutated after registration: #[opt] requires GodotImmutable types and runs them through
// opt_default_value(), which makes engine containers read-only. Concurrent readers can thus only clone them (through atomic Godot refcounts).
unsafe impl Sync for MethodDefaults {}

impl std::ops::Deref for MethodDefaults {
    type Target = [Variant];

    fn deref(&self) -> &[Variant] {
        &self.0
    }
}

/// Varcall FFI entry point shared by all `#[func]`s with signature `(Params, Ret)`.
///
/// Applies default arguments when the caller provides fewer than the method declares. Registered for every `#[func]` alongside
/// [`ptrcall_callback()`]. Godot picks the convention per call, based on the type information available at the call site.
///
/// # Safety
/// `method_data` must point to a `MethodUserdata<Params, Ret>` stored by [`ClassMethodInfo::from_signature()`]; the remaining parameters must
/// follow the varcall convention for that signature.
unsafe extern "C" fn varcall_callback<Params: InParamTuple, Ret: EngineToGodot>(
    method_data: *mut c_void,
    instance_ptr: sys::GDExtensionClassInstancePtr,
    args_ptr: *const sys::GDExtensionConstVariantPtr,
    arg_count: sys::GDExtensionInt,
    ret: sys::GDExtensionVariantPtr,
    err: *mut sys::GDExtensionCallError,
) {
    // SAFETY: `method_data` is the pointer registered together with this function, and the data behind it is never mutated, nor freed while
    // the method stays registered.
    let data = unsafe { &*method_data.cast::<MethodUserdata<Params, Ret>>() };
    let call_ctx = CallContext::func(data.header.class_name, data.header.method_name);

    let code = || {
        // SAFETY: guaranteed by this function's caller.
        unsafe {
            Signature::<Params, Ret>::in_varcall(
                instance_ptr,
                &call_ctx,
                args_ptr,
                arg_count,
                &data.header.default_arguments,
                ret,
                err,
                data.func,
            )
        }
    };

    // SAFETY: `err` points to a live call error, as guaranteed by the caller.
    unsafe { crate::private::handle_fallible_varcall(&call_ctx, err, code) };
}

/// Ptrcall FFI entry point shared by all `#[func]`s with signature `(Params, Ret)`.
///
/// Faster path without default-argument handling, usable once the caller provides all arguments. Registered for every `#[func]`, also for
/// those declaring `#[opt]` defaults; see [`varcall_callback()`].
///
/// # Safety
/// `method_data` must point to a `MethodUserdata<Params, Ret>` stored by [`ClassMethodInfo::from_signature()`]; the remaining parameters must
/// follow the ptrcall convention for that signature.
unsafe extern "C" fn ptrcall_callback<Params: InParamTuple, Ret: EngineToGodot>(
    method_data: *mut c_void,
    instance_ptr: sys::GDExtensionClassInstancePtr,
    args_ptr: *const sys::GDExtensionConstTypePtr,
    ret: sys::GDExtensionTypePtr,
) {
    // SAFETY: `method_data` is the pointer registered together with this function, and the data behind it is never mutated, nor freed while
    // the method stays registered.
    let data = unsafe { &*method_data.cast::<MethodUserdata<Params, Ret>>() };
    let call_ctx = CallContext::func(data.header.class_name, data.header.method_name);

    let code = || {
        // SAFETY: guaranteed by this function's caller.
        unsafe {
            Signature::<Params, Ret>::in_ptrcall(
                instance_ptr,
                &call_ctx,
                args_ptr,
                ret,
                data.func,
                sys::PtrcallType::Standard,
            )
        }
    };

    crate::private::handle_fallible_ptrcall(&call_ctx, code);
}
