/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// use crate::GodotClass;
// use godot_core::Variant;
// use godot_ffi as sys;
// use std::borrow::Cow;
// use std::ffi::CStr;

// pub trait Method<C> {
//     type ReturnType;
//     type ParamTypes;
//
//     fn method_name(&self) -> Cow<CStr>;
//     fn ptrcall(&mut self, instance: &mut C, args: Self::ParamTypes) -> Self::ReturnType;
// }

// ----------------------------------------------------------------------------------------------------------------------------------------------
/*

/// Method known at compile time (statically), either a Rust `fn` or closure.
pub trait CodeMethod<C, R, Ps>
where
    C: GodotClass,
{
    const PARAM_COUNT: usize;
    const NAME: &'static str;

    unsafe fn varcall(
        &mut self,
        instance: sys::GDExtensionClassInstancePtr,
        args: *const sys::GDExtensionTypePtr,
        ret: sys::GDExtensionTypePtr,
        err: *mut sys::GDExtensionCallError,
    );

    unsafe fn ptrcall(
        &mut self,
        instance: sys::GDExtensionClassInstancePtr,
        args: *const sys::GDExtensionTypePtr,
        ret: sys::GDExtensionTypePtr,
    );
}


// TODO code duplication ((2))
macro_rules! count_idents {
    () => {
        0
    };
    ($name:ident, $($other:ident,)*) => {
        1 + $crate::gdext_count_idents!($($other,)*)
    }
}

macro_rules! impl_code_method {
// 	( $( $Param:ident ),* ) => {
    ( $( $Param:ident $arg:ident ),* ) => {
        impl<C, F, R, $( $Param ),*> CodeMethod<C, R, ( $( $Param, )* )> for F
        where
            C: $crate::GodotClass + $crate::GodotDefault, // TODO only GodotClass
            F: Fn(&C, $( $Param ),* ) -> R,
            $(
                $Param: sys::GodotFfi,
            )*
            R: sys::GodotFfi + 'static,
        {
            const PARAM_COUNT: usize = count_idents!($( $Param, )*);

            // Varcall
            #[inline]
            #[allow(unused_variables, unused_assignments, unused_mut)]
            unsafe fn varcall(
                &mut self,
                instance: sys::GDExtensionClassInstancePtr,
                args: *const sys::GDExtensionTypePtr,
                ret: sys::GDExtensionTypePtr,
                err: *mut sys::GDExtensionCallError,
            ) {
                let storage = ::godot_core::private::as_storage::<C>(instance);
                let instance = storage.get_mut_lateinit();

                let mut idx = 0;

                $(
                    let $arg = <$Param as FromVariant::from_variant(&*(*args.offset(idx) as *mut Variant));
                    idx += 1;
                )*

                let ret_val = self(&instance, $(
                    $arg,
                )*);

                *(ret as *mut Variant) = Variant::from(ret_val);
                (*err).error = sys::GDExtensionCallErrorType_GDEXTENSION_CALL_OK;
            }


            // Ptrcall
            #[inline]
            #[allow(unused_variables, unused_assignments, unused_mut)]
            unsafe fn ptrcall(
                &mut self,
                instance: sys::GDExtensionClassInstancePtr,
                args: *const sys::GDExtensionTypePtr,
                ret: sys::GDExtensionTypePtr,
            ) {
                let storage = $crate::private::as_storage::<C>(instance);
                let instance = storage.get_mut_lateinit();

                // TODO reuse code, see ((1))
                let mut idx = 0;

                $(
                    let $arg = <$Param as sys::GodotFfi>::from_sys(*args.offset(idx));
                    // FIXME update refcount, e.g. Gd::ready() or T::Mem::maybe_inc_ref(&result);
                    // possibly in from_sys() directly; what about from_sys_init() and from_{obj|str}_sys()?
                    idx += 1;
                )*

                let ret_val = self(&instance, $(
                    $arg,
                )*);

                <R as sys::GodotFfi>::write_sys(&ret_val, ret);
            }
        }

    };
}

impl_code_method!();
impl_code_method!(P0 arg0);
impl_code_method!(P0 arg0, P1 arg1);
impl_code_method!(P0 arg0, P1 arg1, P2 arg2);
*/
