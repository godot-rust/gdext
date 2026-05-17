/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Needed for Clippy to accept #[cfg(all())].
#![allow(clippy::non_minimal_cfg)]

use godot::builtin::vslice;
use godot::classes::ClassDb;
use godot::obj::Singleton;
use godot::prelude::*;

use crate::framework::{expect_panic, itest};

#[derive(GodotClass)]
#[class(init, base=RefCounted)]
struct FuncObj;

#[godot_api]
impl FuncObj {
    #[func(rename=is_true)]
    fn long_function_name_for_is_true(&self) -> bool {
        true
    }

    #[func(rename=give_one)]
    fn give_one_inner(&self) -> i32 {
        self.give_one()
    }

    #[func(rename=spell_static)]
    fn renamed_static() -> GString {
        GString::from("static")
    }

    #[cfg(all())]
    fn returns_hello_world(&self) -> GString {
        GString::from("Hello world!")
    }

    #[cfg(any())]
    fn returns_hello_world(&self) -> GString {
        compile_error!("Removed by #[cfg]")
    }

    #[cfg(any())]
    fn returns_bye_world(&self) -> GString {
        compile_error!("Removed by #[cfg]")
    }

    #[cfg(all())]
    fn returns_bye_world(&self) -> GString {
        GString::from("Bye world!")
    }

    #[func]
    fn method_with_defaults(
        &self,
        required: i32,
        #[opt(default = "Default str")] string: GString,
        #[opt(default = 100)] integer: i32,
    ) -> VarArray {
        varray![required, &string, integer]
    }

    #[func]
    fn method_with_immutable_array_default(
        &self,
        #[opt(default = &array![1, 2, 3])] arr: Array<i64>,
    ) -> Array<i64> {
        arr
    }

    /* For now, Gd<T> types cannot be used as default parameters due to immutability requirement.
    #[func]
    fn static_with_defaults(
        #[opt(default = &RefCounted::new_gd())] mut required: Gd<RefCounted>,
        #[opt(default = Gd::null_arg())] nullable: Option<Gd<RefCounted>>,
    ) -> Gd<RefCounted> {
        let id = match nullable {
            Some(obj) => obj.instance_id().to_i64(),
            None => -1,
        };

        required.set_meta("nullable_id", &id.to_variant());
        required
    }
    */
}

impl FuncObj {
    /// Unused but present to demonstrate how `rename = ...` can be used to avoid name clashes.
    #[allow(dead_code)]
    fn is_true(&self) -> bool {
        false
    }

    fn give_one(&self) -> i32 {
        1
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(GodotClass)]
#[class(base=RefCounted)]
struct GdSelfObj {
    internal_value: i32,

    base: Base<RefCounted>,
}

#[godot_api]
impl GdSelfObj {
    // A signal that will be looped back to update_internal through gdscript.
    #[signal(__no_builder)]
    fn update_internal_signal(new_internal: i32);

    #[func]
    fn update_internal(&mut self, new_value: i32) {
        self.internal_value = new_value;
    }

    #[func]
    #[rustfmt::skip]
    fn func_shouldnt_panic_with_segmented_path_attribute() -> bool {
        true
    }

    #[cfg(all())]
    #[func]
    fn func_recognized_with_simple_path_attribute_above_func_attr() -> bool {
        true
    }

    #[func]
    #[cfg(all())]
    fn func_recognized_with_simple_path_attribute_below_func_attr() -> bool {
        true
    }

    #[func]
    fn funcs_above_are_kept() -> bool {
        let f2 = Self::func_recognized_with_simple_path_attribute_above_func_attr();
        let f1 = Self::func_recognized_with_simple_path_attribute_below_func_attr();

        f1 && f2
    }

    #[func]
    fn cfg_removes_duplicate_function_impl() -> bool {
        true
    }

    #[cfg(any())]
    #[func]
    fn cfg_removes_duplicate_function_impl() -> bool {
        compile_error!("Removed by #[cfg]")
    }

    #[func]
    #[cfg(any())]
    fn cfg_removes_duplicate_function_impl() -> bool {
        compile_error!("Removed by #[cfg]")
    }

    // Why `panic = "abort"`: we need a condition that always evaluates to true, and #[cfg_attr(true)] is still experimental.
    // (https://github.com/rust-lang/rust/issues/131204)
    #[cfg_attr(any(panic = "abort", panic = "unwind"), cfg(any()))]
    #[func]
    fn cfg_removes_duplicate_function_impl() -> bool {
        compile_error!("Removed by #[cfg]")
    }

    #[func]
    // Why `panic = "abort"`: we need a condition that always evaluates to true, and #[cfg_attr(true)] is still experimental.
    // (https://github.com/rust-lang/rust/issues/131204)
    #[cfg_attr(any(panic = "abort", panic = "unwind"), cfg(any()))]
    fn cfg_removes_duplicate_function_impl() -> bool {
        compile_error!("Removed by #[cfg]")
    }

    #[cfg(any())]
    #[func]
    fn cfg_removes_function() -> bool {
        compile_error!("Removed by #[cfg]")
    }

    #[func]
    #[cfg(any())]
    fn cfg_removes_function() -> bool {
        compile_error!("Removed by #[cfg]")
    }

    #[signal]
    #[rustfmt::skip]
    fn signal_shouldnt_panic_with_segmented_path_attribute();

    #[cfg(all())]
    #[signal]
    fn signal_recognized_with_simple_path_attribute_above_signal_attr();

    #[signal]
    #[cfg(all())]
    fn signal_recognized_with_simple_path_attribute_below_signal_attr();

    #[signal]
    fn cfg_removes_duplicate_signal();

    #[cfg(any())]
    #[signal]
    fn cfg_removes_duplicate_signal();

    #[signal]
    #[cfg(any())]
    fn cfg_removes_duplicate_signal();

    #[cfg(any())]
    #[signal]
    fn cfg_removes_signal();

    #[signal]
    #[cfg(any())]
    fn cfg_removes_signal();

    /// Sample docstring.
    ///
    /// Impossible to check by other means than manually, but it is still nice to have some documentation.
    #[signal]
    fn docstring_is_preserved_in_signal();

    /// Sample docstring, to watch if it causes any issues with `#[cfg(...)]`.
    #[signal]
    #[cfg(any())]
    fn cfg_removes_signal_with_docstring();

    #[func]
    fn fail_to_update_internal_value_due_to_conflicting_borrow(
        &mut self,
        new_internal: i32,
    ) -> i32 {
        // Since a self reference is held while the signal is emitted, when
        // GDScript tries to call update_internal(), there will be a failure due
        // to the double borrow and self.internal_value won't be changed.
        self.base_mut()
            .emit_signal("update_internal_signal", vslice![new_internal]);
        self.internal_value
    }

    #[func(gd_self)]
    fn succeed_at_updating_internal_value(mut this: Gd<Self>, new_internal: i32) -> i32 {
        // Since this isn't bound while the signal is emitted, GDScript will succeed at calling
        // update_internal() and self.internal_value will be changed.
        this.emit_signal("update_internal_signal", vslice![new_internal]);

        this.bind().internal_value
    }

    #[func(gd_self)]
    fn takes_gd_as_equivalent(mut this: Gd<GdSelfObj>) -> bool {
        this.bind_mut();
        true
    }

    #[func(gd_self)]
    fn takes_gd_as_self_no_return_type(this: Gd<GdSelfObj>) {
        this.bind();
    }
}

#[godot_api]
impl IRefCounted for GdSelfObj {
    fn init(base: Base<Self::Base>) -> Self {
        Self {
            internal_value: 0,
            base,
        }
    }

    #[cfg(any())]
    fn init(base: Base<Self::Base>) -> Self {
        compile_error!("Removed by #[cfg]")
    }

    #[cfg(all())]
    fn to_string(&self) -> GString {
        GString::new()
    }

    #[cfg(any())]
    fn register_class() {
        compile_error!("Removed by #[cfg]");
    }

    #[cfg(all())]
    fn on_notification(&mut self, _: godot::classes::notify::ObjectNotification) {
        // Do nothing.
    }

    #[cfg(any())]
    fn on_notification(&mut self, _: godot::classes::notify::ObjectNotification) {
        compile_error!("Removed by #[cfg]");
    }

    #[cfg(any())]
    fn cfg_removes_this() {
        compile_error!("Removed by #[cfg]");
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

// Also tests lack of #[class].
#[derive(GodotClass)]
struct InitPanic;

#[godot_api]
impl IRefCounted for InitPanic {
    // Panicking constructor.
    fn init(_base: Base<Self::Base>) -> Self {
        panic!("InitPanic::init() exploded");
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Tests

#[itest]
fn init_panic_is_caught() {
    expect_panic("default construction propagates panic", || {
        let _obj = InitPanic::new_gd();
    });
}

#[itest]
fn init_fn_panic_is_caught() {
    expect_panic("Gd::from_init_fn() propagates panic", || {
        let _obj = Gd::<InitPanic>::from_init_fn(|_base| panic!("custom init closure exploded"));
    });
}

#[itest]
fn func_default_parameters() {
    let mut obj = FuncObj::new_gd();

    let a = obj.call("method_with_defaults", vslice![0]);
    assert_eq!(a.to::<VarArray>(), varray![0, "Default str", 100]);

    let b = obj.call("method_with_defaults", vslice![1, "My string"]);
    assert_eq!(b.to::<VarArray>(), varray![1, "My string", 100]);

    let c = obj.call("method_with_defaults", vslice![2, "Another string", 456]);
    assert_eq!(c.to::<VarArray>(), varray![2, "Another string", 456]);

    /* For now, Gd<T> defaults are disabled due to immutability.
    // Test that object is passed through, and that Option<Gd> with default Gd::null_arg() works.
    let first = RefCounted::new_gd();
    let d = obj
        .call("static_with_defaults", vslice![&first])
        .to::<Gd<RefCounted>>();
    assert_eq!(d.instance_id(), first.instance_id());
    assert_eq!(d.get_meta("nullable_id"), (-1).to_variant());

    // Test that Option<Gd> with a populated argument works.
    let second = RefCounted::new_gd();
    let e = obj
        .call("static_with_defaults", vslice![&first, &second])
        .to::<Gd<RefCounted>>();
    assert_eq!(e.instance_id(), first.instance_id());
    assert_eq!(e.get_meta("nullable_id"), second.instance_id().to_variant());
    */
}

/* For now, Gd<T> defaults are disabled due to immutability.
#[itest]
fn func_defaults_re_evaluate_expr() {
    // ClassDb::class_call_static() added in Godot 4.4, but non-static dispatch works even before.
    #[cfg(since_api = "4.4")]
    let call_api = || -> InstanceId {
        let variant =
            ClassDb::singleton().class_call_static("FuncObj", "static_with_defaults", &[]);
        variant.object_id().unwrap()
    };

    #[cfg(before_api = "4.4")]
    let call_api = || -> InstanceId {
        let variant = FuncObj::new_gd().call("static_with_defaults", &[]);
        variant.object_id().unwrap()
    };

    let first_id = call_api();
    let second_id = call_api();

    assert_ne!(
        first_id, second_id,
        "#[opt = EXPR] should create evaluate EXPR on each call"
    );
}
*/

#[itest]
fn func_immutable_defaults() {
    let mut obj = FuncObj::new_gd();

    // Test Array<T> default parameter.
    let arr = obj
        .call("method_with_immutable_array_default", &[])
        .to::<Array<i64>>();
    assert_eq!(arr, array![1, 2, 3]);

    assert!(
        arr.is_read_only(),
        "GodotImmutable trait did its job to make array read-only"
    );
}

#[itest]
fn cfg_doesnt_interfere_with_valid_method_impls() {
    // If we re-implement this method but the re-implementation is removed, that should keep the non-removed implementation.
    let object = Gd::from_object(FuncObj);
    assert_eq!(
        object.bind().returns_hello_world(),
        GString::from("Hello world!")
    );
    assert_eq!(
        object.bind().returns_bye_world(),
        GString::from("Bye world!")
    );
}

#[itest]
fn cfg_removes_or_keeps_methods() {
    assert!(class_has_method::<GdSelfObj>(
        "func_recognized_with_simple_path_attribute_above_func_attr"
    ));
    assert!(class_has_method::<GdSelfObj>(
        "func_recognized_with_simple_path_attribute_below_func_attr"
    ));
    assert!(class_has_method::<GdSelfObj>(
        "cfg_removes_duplicate_function_impl"
    ));
    assert!(!class_has_method::<GdSelfObj>("cfg_removes_function"));
}

#[itest]
fn cfg_removes_or_keeps_signals() {
    assert!(class_has_signal::<GdSelfObj>(
        "signal_recognized_with_simple_path_attribute_above_signal_attr"
    ));
    assert!(class_has_signal::<GdSelfObj>(
        "signal_recognized_with_simple_path_attribute_below_signal_attr"
    ));
    assert!(class_has_signal::<GdSelfObj>(
        "cfg_removes_duplicate_signal"
    ));
    assert!(!class_has_signal::<GdSelfObj>("cfg_removes_signal"));
}

// No test for Gd::from_object(), as that simply moves the existing object without running user code.

// ----------------------------------------------------------------------------------------------------------------------------------------------
// #[func(lossy)]

#[derive(GodotClass)]
#[class(init, base=RefCounted)]
struct LossyFuncObj;

#[godot_api]
impl LossyFuncObj {
    #[func(lossy)]
    fn double_usize(&self, n: usize) -> usize {
        n.saturating_mul(2)
    }

    // Returns usize::MAX -- overflows i64 on 64-bit targets; on wasm32 (32-bit usize) fits in i64.
    #[func(lossy)]
    fn return_usize_max(&self) -> usize {
        usize::MAX
    }

    // Echoes a u64 directly. Used to test both successful (small) and overflow (> i64::MAX) returns.
    #[func(lossy)]
    fn echo_u64(&self, n: u64) -> u64 {
        n
    }

    // Returns u64::MAX -- always overflows i64::MAX, regardless of target.
    #[func(lossy)]
    fn return_u64_max(&self) -> u64 {
        u64::MAX
    }

    // Mixed lossy + non-lossy params (arity 2): exercises LossyTupleFromGodot beyond arity 1.
    #[func(lossy)]
    fn concat_indexed(&self, prefix: GString, n: usize) -> GString {
        GString::from(&format!("{prefix}:{n}"))
    }
}

#[itest]
fn func_lossy_usize_compute() {
    let mut obj = LossyFuncObj::new_gd();
    let result = obj.call("double_usize", &[21_i64.to_variant()]);
    assert_eq!(result.to::<i64>(), 42);
}

#[itest]
fn func_lossy_usize_negative_input_fails() {
    // GDScript can pass negative i64; usize input rejects negatives target-agnostically.
    let mut obj = LossyFuncObj::new_gd();
    let result = obj.try_call("double_usize", &[(-1_i64).to_variant()]);
    assert!(result.is_err());
}

#[itest]
#[cfg(target_pointer_width = "64")]
fn func_lossy_usize_return_overflow_fails() {
    // usize::MAX = u64::MAX on 64-bit targets -> doesn't fit i64, must surface as CallError.
    let mut obj = LossyFuncObj::new_gd();
    let result = obj.try_call("return_usize_max", &[]);
    assert!(result.is_err());
}

#[itest]
fn func_lossy_u64_roundtrip_small() {
    // Small u64 values fit i64; should round-trip.
    let mut obj = LossyFuncObj::new_gd();
    let result = obj.call("echo_u64", &[7_i64.to_variant()]);
    assert_eq!(result.to::<i64>(), 7);
}

#[itest]
fn func_lossy_u64_negative_input_reinterprets() {
    // Documented semantic: u64 input is bit-reinterpret (engine API contract).
    // Negative GDScript int -> high-bit-set u64. On return, that u64 > i64::MAX -> CallError.
    let mut obj = LossyFuncObj::new_gd();
    let result = obj.try_call("echo_u64", &[(-1_i64).to_variant()]);
    assert!(result.is_err());
}

#[itest]
fn func_lossy_u64_return_overflow_fails() {
    let mut obj = LossyFuncObj::new_gd();
    let result = obj.try_call("return_u64_max", &[]);
    assert!(result.is_err());
}

#[itest]
fn func_lossy_mixed_params() {
    // Verifies LossyTupleFromGodot arity > 1 + mix of FromGodot (GString) and lossy (usize) params.
    let mut obj = LossyFuncObj::new_gd();
    let args = ["item".to_variant(), 7_i64.to_variant()];
    let result = obj.call("concat_indexed", &args);
    assert_eq!(result.to::<GString>(), GString::from("item:7"));
}

#[itest]
fn func_lossy_usize_non_int_variant_fails() {
    // Non-int Variant for usize param exercises engine_try_from_variant path: i64 extraction fails -> CallError.
    let mut obj = LossyFuncObj::new_gd();
    let result = obj.try_call("double_usize", &["not_a_number".to_variant()]);
    assert!(result.is_err());
}

#[itest]
#[cfg(target_pointer_width = "32")]
fn func_lossy_usize_wasm_above_u32_max_fails() {
    // wasm32-only: usize is 32-bit, so i64 values above u32::MAX must be rejected on input.
    let mut obj = LossyFuncObj::new_gd();
    let big = (u32::MAX as i64) + 1;
    let result = obj.try_call("double_usize", &[big.to_variant()]);
    assert!(result.is_err());
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Helpers

/// Checks at runtime if a class has a given method through [ClassDb].
fn class_has_method<T: GodotClass>(name: &str) -> bool {
    ClassDb::singleton()
        .class_has_method_ex(&T::class_id().to_string_name(), name)
        .no_inheritance(true)
        .done()
}

/// Checks at runtime if a class has a given signal through [ClassDb].
fn class_has_signal<T: GodotClass>(name: &str) -> bool {
    ClassDb::singleton().class_has_signal(&T::class_id().to_string_name(), name)
}
