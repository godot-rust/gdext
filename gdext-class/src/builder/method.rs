use crate::GodotClass;
use gdext_sys as sys;

pub trait Method<Class>
where
    Class: GodotClass,
{
    unsafe fn ptrcall(
        &mut self,
        instance: sys::GDExtensionClassInstancePtr,
        args: *const sys::GDNativeTypePtr,
        ret: sys::GDNativeTypePtr,
    );
}

/// Method known at compile time (statically), either a Rust `fn` or closure.
pub trait RustMethod<Class, Ret, Params>
where
    Class: GodotClass,
{
    unsafe fn ptrcall_rust(
        &mut self,
        instance: sys::GDExtensionClassInstancePtr,
        args: *const sys::GDNativeTypePtr,
        ret: sys::GDNativeTypePtr,
    );
}

// impl<Class, F, Ret, Params> Method<Class> for F
// where
//     F: RustMethod<Class, Ret, Params>,
// {
//     unsafe fn ptrcall(
//         &mut self,
//         instance: sys::GDExtensionClassInstancePtr,
//         args: *const sys::GDNativeTypePtr,
//         ret: sys::GDNativeTypePtr,
//     ) {
//         //<Self as RustMethod<Class, Ret, Params>>::ptrcall_rust(self, instance, args, ret)
//         self.ptrcall_rust(instance, args, ret)
//     }
// }

macro_rules! impl_rust_method {
// 	( $( $Param:ident ),* ) => {
    ( $( $Param:ident $arg:ident ),* ) => {
		impl<C, F, R, $( $Param ),*> RustMethod<C, R, ( $( $Param, )* )> for F
		where
			C: $crate::GodotClass + $crate::GodotDefault, // TODO only GodotClass
			F: Fn(&C, $( $Param ),* ) -> R,
			$(
				$Param: sys::GodotFfi,
			)*
			R: sys::GodotFfi + 'static,
		{
			#[allow(unused_variables, unused_assignments, unused_mut)]
			unsafe fn ptrcall_rust(
				&mut self,
				instance: sys::GDExtensionClassInstancePtr,
				args: *const sys::GDNativeTypePtr,
				ret: sys::GDNativeTypePtr,
			) {
				let storage = $crate::private::as_storage::<C>(instance);
				let instance = storage.get_mut_lateinit();

				// TODO reuse code, see ((1))
				let mut idx = 0;

				$(
					let $arg = <$Param as sys::GodotFfi>::from_sys(*args.offset(idx));
					// FIXME update refcount, e.g. Obj::ready() or T::Mem::maybe_inc_ref(&result);
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

impl_rust_method!();
impl_rust_method!(P0 arg0);
impl_rust_method!(P0 arg0, P1 arg1);
impl_rust_method!(P0 arg0, P1 arg1, P2 arg2);
