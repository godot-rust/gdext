use std::borrow::Cow;
use std::ffi::CStr;
use gdext_builtin::Variant;
use crate::GodotClass;
use gdext_sys as sys;

pub trait Method<C> {
    type ReturnType;
    type ParamTypes;

	fn method_name(&self) -> Cow<CStr>;
	fn ptrcall(&mut self, instance: &mut C, args: Self::ParamTypes) -> Self::ReturnType;
}


fn register_method<C,M,R,Ps>(method: M) where C: GodotClass, M: CodeMethod<C, R, Ps> {

	//let class_name

	let method_info = sys::GDNativeExtensionClassMethodInfo {
		name: concat!(stringify!($method_name), "\0").as_ptr() as *const i8,
		method_userdata: std::ptr::null_mut(),
		call_func: Some({
			unsafe extern "C" fn call(
				_method_data: *mut std::ffi::c_void,
				instance: sys::GDExtensionClassInstancePtr,
				args: *const sys::GDNativeVariantPtr,
				_arg_count: sys::GDNativeInt,
				ret: sys::GDNativeVariantPtr,
				err: *mut sys::GDNativeCallError,
			) {
				method.varcall_fn(instance, args, ret, err);
			}

			call
		}),
		ptrcall_func: Some({
			unsafe extern "C" fn call(
				_method_data: *mut std::ffi::c_void,
				instance: sys::GDExtensionClassInstancePtr,
				args: *const sys::GDNativeTypePtr,
				ret: sys::GDNativeTypePtr,
			) {
				method.ptrcall_fn(instance, args, ret, err);
			}

			call
		}),
		method_flags:
		sys::GDNativeExtensionClassMethodFlags_GDNATIVE_EXTENSION_METHOD_FLAGS_DEFAULT as u32,
		argument_count: M::PARAM_COUNT as u32,
		has_return_value: (std::any::type_name::<R>() == std::any::type_name::<()>()) as u8, // TODO compile-time
		get_argument_type_func: Some({
			extern "C" fn get_type(
				_method_data: *mut std::ffi::c_void,
				n: i32,
			) -> sys::GDNativeVariantType {
				// Return value is the first "argument"




				let types: [sys::GDNativeVariantType; NUM_ARGS + 1] = [
					<$($retty)+ as $crate::property_info::PropertyInfoBuilder>::variant_type(),
				$(
					<$pty as $crate::property_info::PropertyInfoBuilder>::variant_type(),
				)*
				];
				types[(n + 1) as usize]
			}
			get_type
		}),
		get_argument_info_func: Some({
			unsafe extern "C" fn get_info(
				_method_data: *mut std::ffi::c_void,
				n: i32,
				ret: *mut sys::GDNativePropertyInfo,
			) {
				// Return value is the first "argument"
				let infos: [sys::GDNativePropertyInfo; NUM_ARGS + 1] = [
					<$($retty)+ as $crate::property_info::PropertyInfoBuilder>::property_info(""),
				$(
					<$pty as $crate::property_info::PropertyInfoBuilder>::property_info(stringify!($pname)),
				)*
				];

				*ret = infos[(n + 1) as usize];
			}
			get_info
		}),
		get_argument_metadata_func: Some({
			extern "C" fn get_meta(
				_method_data: *mut std::ffi::c_void,
				n: i32,
			) -> sys::GDNativeExtensionClassMethodArgumentMetadata {
				// Return value is the first "argument"
				let metas: [sys::GDNativeExtensionClassMethodArgumentMetadata; NUM_ARGS + 1] = [
					<R as $crate::property_info::PropertyInfoBuilder>::metadata(),
				$(
					<$pty as $crate::property_info::PropertyInfoBuilder>::metadata(),
				)*
				];
				metas[(n + 1) as usize]
			}
			get_meta
		}),
		default_argument_count: 0,
		default_arguments: std::ptr::null_mut(),
	};

	let name = std::ffi::CStr::from_bytes_with_nul_unchecked(concat!(stringify!($type_name), "\0").as_bytes());

	sys::interface_fn!(classdb_register_extension_class_method)(
		sys::get_library(),
		name.as_ptr(),
		std::ptr::addr_of!(method_info),
	);
}

// ----------------------------------------------------------------------------------------------------------------------------------------------


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
		args: *const sys::GDNativeTypePtr,
		ret: sys::GDNativeTypePtr,
		err: *mut sys::GDNativeCallError,
	);

	unsafe fn ptrcall(
		&mut self,
		instance: sys::GDExtensionClassInstancePtr,
		args: *const sys::GDNativeTypePtr,
		ret: sys::GDNativeTypePtr,
	);
}

// TODO code duplication ((2))
macro_rules! count_idents {
    () => {
        0
    };
    ($name:ident, $($other:ident,)*) => {
        1 + $crate::gdext_wrap_method_parameter_count!($($other,)*)
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
			unsafe fn varcall_fn(
				&mut self,
				instance: sys::GDExtensionClassInstancePtr,
				args: *const sys::GDNativeTypePtr,
				ret: sys::GDNativeTypePtr,
				err: *mut sys::GDNativeCallError,
			) {
				let storage = ::gdext_class::private::as_storage::<$type_name>(instance);
				let instance = storage.get_mut_lateinit();

				let mut idx = 0;

				$(
					let $arg = <$Param as From<&Variant>>::from(&*(*args.offset(idx) as *mut Variant));
					idx += 1;
				)*

				let ret_val = self(&instance, $(
					$arg,
				)*);

				*(ret as *mut Variant) = Variant::from(ret_val);
				(*err).error = sys::GDNativeCallErrorType_GDNATIVE_CALL_OK;
			}


			// Ptrcall
			#[inline]
			#[allow(unused_variables, unused_assignments, unused_mut)]
			unsafe fn ptrcall_fn(
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

impl_code_method!();
impl_code_method!(P0 arg0);
impl_code_method!(P0 arg0, P1 arg1);
impl_code_method!(P0 arg0, P1 arg1, P2 arg2);
