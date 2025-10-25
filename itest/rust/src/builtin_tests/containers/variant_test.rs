/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cmp::Ordering;
use std::fmt;
use std::fmt::Display;

use godot::builtin::{
    array, varray, vdict, vslice, Array, Basis, Color, Dictionary, GString, NodePath,
    PackedInt32Array, PackedStringArray, Projection, Quaternion, Signal, StringName, Transform2D,
    Transform3D, Variant, VariantArray, VariantOperator, VariantType, Vector2, Vector2i, Vector3,
    Vector3i,
};
use godot::classes::{Node, Node2D, Resource};
use godot::meta::{FromGodot, ToGodot};
use godot::obj::{Gd, InstanceId, NewAlloc, NewGd};
use godot::sys::GodotFfi;

use crate::common::roundtrip;
use crate::framework::{expect_panic, expect_panic_or_ub, itest, runs_release};

const TEST_BASIS: Basis = Basis::from_rows(
    Vector3::new(1.0, 2.0, 3.0),
    Vector3::new(4.0, 5.0, 6.0),
    Vector3::new(7.0, 8.0, 9.0),
);

#[itest]
fn variant_nil() {
    let variant = Variant::nil();
    assert!(variant.is_nil());

    let variant = 12345i32.to_variant();
    assert!(!variant.is_nil());
}

#[itest]
fn variant_conversions() {
    roundtrip(false);
    roundtrip(true);
    roundtrip(InstanceId::from_i64(-9223372036854775808i64));
    // roundtrip(Some(InstanceId::from_i64(9223372036854775807i64)));
    // roundtrip(Option::<InstanceId>::None);

    // unsigned
    roundtrip(0u8);
    roundtrip(255u8);
    roundtrip(0u16);
    roundtrip(65535u16);
    roundtrip(0u32);
    roundtrip(4294967295u32);

    // signed
    roundtrip(127i8);
    roundtrip(-128i8);
    roundtrip(32767i16);
    roundtrip(-32768i16);
    roundtrip(2147483647i32);
    roundtrip(-2147483648i32);
    roundtrip(9223372036854775807i64);

    // string
    roundtrip(gstr("some string"));
    roundtrip(String::from("some other string"));
    let str_val = "abcdefghijklmnop";
    let back = String::from_variant(&str_val.to_variant());
    assert_eq!(str_val, back.as_str());

    // basis
    roundtrip(TEST_BASIS);

    // signal
    roundtrip(Signal::invalid());
}

#[itest]
fn variant_relaxed_conversions() {
    // See https://github.com/godotengine/godot/blob/4.4-stable/core/variant/variant.cpp#L532.

    let obj = Node::new_alloc();

    // reflexive
    convert_relaxed_to(-22i8, -22i8);
    convert_relaxed_to("some str", GString::from("some str"));
    convert_relaxed_to(TEST_BASIS, TEST_BASIS);
    convert_relaxed_to(obj.clone(), obj.clone());

    // int <-> float
    convert_relaxed_to(1234567890i64, 1234567890f64);
    convert_relaxed_to(1234567890f64, 1234567890i64);

    // int <-> bool
    convert_relaxed_to(-123, true);
    convert_relaxed_to(0, false);
    convert_relaxed_to(true, 1);
    convert_relaxed_to(false, 0);

    // float <-> bool
    convert_relaxed_to(123.45, true);
    convert_relaxed_to(0.0, false);
    convert_relaxed_to(true, 1.0);
    convert_relaxed_to(false, 0.0);

    // GString <-> StringName
    convert_relaxed_to("hello", StringName::from("hello"));
    convert_relaxed_to(StringName::from("hello"), GString::from("hello"));

    // GString <-> NodePath
    convert_relaxed_to("hello", NodePath::from("hello"));
    convert_relaxed_to(NodePath::from("hello"), GString::from("hello"));

    // anything -> nil
    convert_relaxed_to(Variant::nil(), Variant::nil());
    convert_relaxed_to((), Variant::nil());
    convert_relaxed_fail::<()>(obj.clone());
    convert_relaxed_fail::<()>(123.45);
    convert_relaxed_fail::<()>(Vector3i::new(1, 2, 3));

    // nil -> anything (except Variant) - fails
    convert_relaxed_fail::<i64>(Variant::nil());
    convert_relaxed_fail::<GString>(Variant::nil());
    convert_relaxed_fail::<Gd<Node>>(Variant::nil());
    convert_relaxed_fail::<VariantArray>(Variant::nil());
    convert_relaxed_fail::<Dictionary>(Variant::nil());

    // anything -> Variant
    convert_relaxed_to(123, Variant::from(123));
    convert_relaxed_to("hello", Variant::from("hello"));

    // Array -> PackedArray
    let packed_ints = PackedInt32Array::from([1, 2, 3]);
    let packed_strings = PackedStringArray::from(["a".into(), "bb".into()]);
    let strings: Array<GString> = array!["a", "bb"];

    convert_relaxed_to(array![1, 2, 3], packed_ints.clone());
    convert_relaxed_to(varray![1, 2, 3], packed_ints.clone());
    convert_relaxed_to(strings.clone(), packed_strings.clone());
    convert_relaxed_to(varray!["a", "bb"], packed_strings.clone());

    // Packed*Array -> Array
    convert_relaxed_to(packed_ints.clone(), array![1, 2, 3]);
    convert_relaxed_to(packed_ints, varray![1, 2, 3]);
    convert_relaxed_to(packed_strings.clone(), strings);
    convert_relaxed_to(packed_strings, varray!["a", "bb"]);

    // Object|nil -> optional Object
    convert_relaxed_to(obj.clone(), Some(obj.clone()));
    convert_relaxed_to(Variant::nil(), Option::<Gd<Node>>::None);

    // Object -> Rid
    let res = Resource::new_gd();
    let rid = res.get_rid();
    convert_relaxed_to(res.clone(), rid);

    // Vector2 <-> Vector2i
    convert_relaxed_to(Vector2::new(1.0, 2.0), Vector2i::new(1, 2));
    convert_relaxed_to(Vector2i::new(1, 2), Vector2::new(1.0, 2.0));

    // int|String -> Color (don't use float colors due to rounding errors / 255-vs-256 imprecision).
    convert_relaxed_to(0xFF_80_00_40u32, Color::from_rgba8(255, 128, 0, 64));
    convert_relaxed_to("MEDIUM_AQUAMARINE", Color::MEDIUM_AQUAMARINE);

    // Everything -> Transform3D
    convert_relaxed_to(Transform2D::IDENTITY, Transform3D::IDENTITY);
    convert_relaxed_to(Basis::IDENTITY, Transform3D::IDENTITY);
    convert_relaxed_to(Quaternion::IDENTITY, Transform3D::IDENTITY);

    // Projection <-> Transform3D
    convert_relaxed_to(Projection::IDENTITY, Transform3D::IDENTITY);
    convert_relaxed_to(Transform3D::IDENTITY, Projection::IDENTITY);

    // Quaternion <-> Basis
    convert_relaxed_to(Basis::IDENTITY, Quaternion::IDENTITY);
    convert_relaxed_to(Quaternion::IDENTITY, Basis::IDENTITY);

    // Other geometric conversions between the above fail.
    convert_relaxed_fail::<Transform2D>(Projection::IDENTITY);
    convert_relaxed_fail::<Transform2D>(Quaternion::IDENTITY);
    convert_relaxed_fail::<Transform2D>(Basis::IDENTITY);
    convert_relaxed_fail::<Projection>(Transform2D::IDENTITY);
    convert_relaxed_fail::<Projection>(Quaternion::IDENTITY);
    convert_relaxed_fail::<Projection>(Basis::IDENTITY);
    convert_relaxed_fail::<Quaternion>(Transform2D::IDENTITY);
    convert_relaxed_fail::<Quaternion>(Projection::IDENTITY);
    convert_relaxed_fail::<Basis>(Transform2D::IDENTITY);
    convert_relaxed_fail::<Basis>(Projection::IDENTITY);

    obj.free();
}

#[itest]
fn variant_bad_integer_conversions() {
    truncate_bad::<i8>(128);
    truncate_bad::<i8>(-129);

    truncate_bad::<u8>(256);
    truncate_bad::<u8>(-1);

    truncate_bad::<i16>(32768);
    truncate_bad::<i16>(-32769);

    truncate_bad::<u16>(65536);
    truncate_bad::<u16>(-1);

    truncate_bad::<i32>(2147483648);
    truncate_bad::<i32>(-2147483649);

    truncate_bad::<u32>(4294967296);
    truncate_bad::<u32>(-1);

    truncate_bad::<u64>(-1);
}

#[itest]
fn variant_bad_conversions() {
    fn assert_convert_err<T: ToGodot, U: FromGodot + std::fmt::Debug>(value: T) {
        use std::any::type_name;
        value.to_variant().try_to::<U>().expect_err(&format!(
            "`{}` should not convert to `{}`",
            type_name::<T>(),
            type_name::<U>()
        ));
    }

    assert_convert_err::<_, i64>("hello");
    assert_convert_err::<i32, f32>(12);
    assert_convert_err::<f32, i32>(1.23);
    assert_convert_err::<i32, bool>(10);
    assert_convert_err::<_, String>(false);
    assert_convert_err::<_, StringName>(VariantArray::default());

    // Special case: ToVariant is not yet fallible, so u64 -> i64 conversion error panics.
    expect_panic("u64 -> i64 conversion error", || {
        u64::MAX.to_variant();
    });

    //assert_eq!(
    //    Dictionary::default().to_variant().try_to::<Array>(),
    //    Err(VariantConversionError)
    //);
    Variant::nil()
        .to_variant()
        .try_to::<Dictionary>()
        .expect_err("`nil` should not convert to `Dictionary`");
}

#[itest]
fn variant_dead_object_conversions() {
    let obj = Node::new_alloc();
    let variant = obj.to_variant();

    let result = variant.try_to::<Gd<Node>>();
    let gd = result.expect("Variant::to() with live object should succeed");
    assert_eq!(gd, obj);

    obj.free();

    // Verify Display + Debug impl.
    assert_eq!(format!("{variant}"), "<Freed Object>");
    assert_eq!(format!("{variant:?}"), "VariantGd { freed obj }");

    // Variant::try_to().
    let result = variant.try_to::<Gd<Node>>();
    let err = result.expect_err("Variant::try_to::<Gd>() with dead object should fail");
    assert_eq!(
        err.to_string(),
        "variant holds object which is no longer alive: VariantGd { freed obj }"
    );

    // Variant::to().
    expect_panic_or_ub("Variant::to() with dead object should panic", || {
        let _: Gd<Node> = variant.to();
    });

    // Variant::try_to() -> Option<Gd>.
    // This conversion does *not* return `None` for dead objects, but an error. `None` is reserved for NIL variants, see object_test.rs.
    let result = variant.try_to::<Option<Gd<Node>>>();
    let err = result.expect_err("Variant::try_to::<Option<Gd>>() with dead object should fail");
    assert_eq!(
        err.to_string(),
        "variant holds object which is no longer alive: VariantGd { freed obj }"
    );
}

#[itest]
fn variant_bad_conversion_error_message() {
    let variant = 123.to_variant();

    let err = variant
        .try_to::<GString>()
        .expect_err("i32 -> GString conversion should fail");
    assert_eq!(err.to_string(), "cannot convert from INT to STRING: 123");

    let err = variant
        .try_to::<Gd<Node>>()
        .expect_err("i32 -> Gd<Node> conversion should fail");
    assert_eq!(err.to_string(), "cannot convert from INT to OBJECT: 123");
}

// Different builtin types: i32 -> Object.
#[itest]
fn variant_array_bad_type_conversions() {
    let i32_array: Array<i32> = array![1, 2, 160, -40];
    let i32_variant = i32_array.to_variant();
    let object_array = i32_variant.try_to::<Array<Gd<Node>>>();

    let err = object_array.expect_err("Array<i32> -> Array<Gd<Node>> conversion should fail");
    assert_eq!(
        err.to_string(),
        "expected array of type Class(Node), got Builtin(INT): [1, 2, 160, -40]"
    )
}

// Incompatible class types: Node -> Node2D.
#[itest]
fn variant_array_bad_class_conversions() {
    // Even empty arrays are typed and cannot be converted.
    let node_array: Array<Gd<Node>> = array![];
    let node_variant = node_array.to_variant();
    let node2d_array = node_variant.try_to::<Array<Gd<Node2D>>>();

    let err =
        node2d_array.expect_err("Array<Gd<Node>> -> Array<Gd<Node2D>> conversion should fail");
    assert_eq!(
        err.to_string(),
        "expected array of type Class(Node2D), got Class(Node): []"
    )
}

// Convert typed to untyped array (incompatible).
#[itest]
fn variant_array_to_untyped_conversions() {
    // Even empty arrays are typed and cannot be converted.
    let node_array: Array<Gd<Node>> = array![];
    let node_variant = node_array.to_variant();
    let untyped_array = node_variant.try_to::<VariantArray>();

    let err = untyped_array.expect_err("Array<Gd<Node>> -> VariantArray conversion should fail");
    assert_eq!(
        err.to_string(),
        "expected array of type Untyped, got Class(Node): []"
    )
}

// Convert typed to untyped array (incompatible).
#[itest]
fn variant_array_from_untyped_conversions() {
    // Even empty arrays are typed and cannot be converted.
    let untyped_array: VariantArray = varray![1, 2];
    let untyped_variant = untyped_array.to_variant();
    let int_array = untyped_variant.try_to::<Array<i64>>();

    let err = int_array.expect_err("VariantArray -> Array<i64> conversion should fail");
    assert_eq!(
        err.to_string(),
        "expected array of type Builtin(INT), got Untyped: [1, 2]"
    )
}

// Same builtin type INT, but incompatible integers (strict-safeguards only).
#[itest]
fn variant_array_bad_integer_conversions() {
    let i32_array: Array<i32> = array![1, 2, 160, -40];
    let i32_variant = i32_array.to_variant();
    let i8_back = i32_variant.try_to::<Array<i8>>();

    // In strict safeguard mode, we expect an error upon conversion.
    #[cfg(safeguards_strict)]
    {
        let err = i8_back.expect_err("Array<i32> -> Array<i8> conversion should fail");
        assert_eq!(
            err.to_string(),
            "integer value 160 does not fit into Array<i8>: [1, 2, 160, -40]"
        )
    }

    // In balanced/disengaged modes, we expect the conversion to succeed, but a panic to occur on element access.
    #[cfg(not(safeguards_strict))]
    {
        let i8_array = i8_back.expect("Array<i32> -> Array<i8> conversion should succeed");
        expect_panic("accessing element 160 as i8 should panic", || {
            // Note: get() returns Err on out-of-bounds, but currently panics on bad element type, since that's always a bug.
            i8_array.get(2);
        });
    }
}

#[itest]
fn variant_special_conversions() {
    // See https://github.com/godot-rust/gdext/pull/598.
    let variant = NodePath::default().to_variant();
    let object = variant.try_to::<Option<Gd<Node>>>();
    assert!(matches!(object, Ok(None)));
}

#[itest]
fn variant_get_type() {
    let variant = Variant::nil();
    assert_eq!(variant.get_type(), VariantType::NIL);

    let variant = 74i32.to_variant();
    assert_eq!(variant.get_type(), VariantType::INT);

    let variant = true.to_variant();
    assert_eq!(variant.get_type(), VariantType::BOOL);

    let variant = gstr("hello").to_variant();
    assert_eq!(variant.get_type(), VariantType::STRING);

    let variant = sname("hello").to_variant();
    assert_eq!(variant.get_type(), VariantType::STRING_NAME);

    let variant = TEST_BASIS.to_variant();
    assert_eq!(variant.get_type(), VariantType::BASIS)
}

#[itest]
fn variant_object_id() {
    let variant = Variant::nil();
    assert_eq!(variant.object_id(), None);

    let variant = Variant::from(77);
    assert_eq!(variant.object_id(), None);

    let node = Node::new_alloc();
    let id = node.instance_id();

    let variant = node.to_variant();
    assert_eq!(variant.object_id(), Some(id));

    node.free();

    // When freed, variant still returns the object ID.
    expect_panic("Variant::object_id() with freed object", || {
        let _ = variant.object_id();
    });
}

#[itest]
#[cfg(since_api = "4.4")]
fn variant_object_id_unchecked() {
    let variant = Variant::nil();
    assert_eq!(variant.object_id_unchecked(), None);

    let variant = Variant::from(77);
    assert_eq!(variant.object_id_unchecked(), None);

    let node = Node::new_alloc();
    let id = node.instance_id();

    let variant = node.to_variant();
    assert_eq!(variant.object_id_unchecked(), Some(id));

    node.free();

    // When freed, unchecked function will still return old ID.
    assert_eq!(variant.object_id_unchecked(), Some(id));
}

#[itest]
fn variant_equal() {
    assert_eq!(Variant::nil(), ().to_variant());
    assert_eq!(Variant::nil(), Variant::default());
    assert_eq!(Variant::from(77), 77.to_variant());

    equal(77, (), false);
    equal(77, 78, false);

    assert_ne!(77.to_variant(), Variant::nil());
    assert_ne!(77.to_variant(), 78.to_variant());

    //equal(77, 77.0, false)
    equal(Vector3::new(1.0, 2.0, 3.0), Vector2::new(1.0, 2.0), false);
    equal(1, true, false);
    equal(false, 0, false);
    equal(gstr("String"), 33, false);
}

#[itest]
fn variant_call() {
    let node2d = Node2D::new_alloc();
    let variant = Variant::from(node2d.clone());

    // Object
    let position = Vector2::new(4.0, 5.0);
    let result = variant.call("set_position", vslice![position]);
    assert!(result.is_nil());

    let result = variant
        .call("get_position", &[])
        .try_to::<Vector2>()
        .expect("`get_position` should return Vector2");
    assert_eq!(result, position);

    let result = variant.call("to_string", &[]);
    assert_eq!(result.get_type(), VariantType::STRING);

    // Array
    let array = godot::builtin::varray![1, "hello", false];
    let result = array.to_variant().call("size", &[]);
    assert_eq!(result, 3.to_variant());

    // String
    let string = GString::from("move_local_x");
    let result = string.to_variant().call("capitalize", &[]);
    assert_eq!(result, "Move Local X".to_variant());

    // Vector2
    let vector = Vector2::new(5.0, 3.0);
    let vector_rhs = Vector2::new(1.0, -1.0);
    let result = vector.to_variant().call("dot", vslice![vector_rhs]);
    assert_eq!(result, 2.0.to_variant());

    // Dynamic checks are only available in Debug builds.
    if !runs_release() {
        expect_panic("Variant::call on non-existent method", || {
            variant.call("gut_position", &[]);
        });
        expect_panic("Variant::call with bad signature", || {
            variant.call("set_position", &[]);
        });
        expect_panic("Variant::call with non-object variant (int)", || {
            Variant::from(77).call("to_string", &[]);
        });
    }

    node2d.free();
}

#[rustfmt::skip]
#[itest]
fn variant_evaluate() {
    evaluate(VariantOperator::ADD, 20, -39, -19);
    evaluate(VariantOperator::GREATER, 20, 19, true);
    evaluate(VariantOperator::EQUAL, 20, 20.0, true);
    evaluate(VariantOperator::NOT_EQUAL, 20, 20.0, false);
    evaluate(VariantOperator::MULTIPLY, 5, 2.5, 12.5);

    evaluate(VariantOperator::EQUAL, gstr("hello"), gstr("hello"), true);
    evaluate(VariantOperator::EQUAL, gstr("hello"), sname("hello"), true);
    evaluate(VariantOperator::EQUAL, sname("rust"), gstr("rust"), true);
    evaluate(VariantOperator::EQUAL, sname("rust"), sname("rust"), true);

    evaluate(VariantOperator::NOT_EQUAL, gstr("hello"), gstr("hallo"), true);
    evaluate(VariantOperator::NOT_EQUAL, gstr("hello"), sname("hallo"), true);
    evaluate(VariantOperator::NOT_EQUAL, sname("rust"), gstr("rest"), true);
    evaluate(VariantOperator::NOT_EQUAL, sname("rust"), sname("rest"), true);

    evaluate_fail(VariantOperator::EQUAL, 1, true);
    evaluate_fail(VariantOperator::EQUAL, 0, false);
    evaluate_fail(VariantOperator::SUBTRACT, 2, Vector3::new(1.0, 2.0, 3.0));
}

#[itest]
fn variant_evaluate_total_order() {
    // See also Godot 4 source: variant_op.cpp

    // NaN incorrect in Godot
    // use VariantOperator::{Equal, Greater, GreaterEqual, Less, LessEqual, NotEqual};
    // for op in [Equal, NotEqual, Less, LessEqual, Greater, GreaterEqual] {
    //     evaluate(op, f64::NAN, f64::NAN, false);
    // }

    total_order(-5, -4, Ordering::Less);
    total_order(-5, -4.0, Ordering::Less);
    total_order(-5.0, -4, Ordering::Less);

    total_order(-5, -5, Ordering::Equal);
    total_order(-5, -5.0, Ordering::Equal);
    total_order(-5.0, -5, Ordering::Equal);

    total_order(gstr("hello"), gstr("hello"), Ordering::Equal);
    total_order(gstr("hello"), gstr("hell"), Ordering::Greater);
}

#[itest]
fn variant_display() {
    let cases = [
        (Variant::nil(), "<null>"),
        (false.to_variant(), "false"),
        (true.to_variant(), "true"),
        (gstr("some string").to_variant(), "some string"),
        //
        // unsigned
        ((0u8).to_variant(), "0"),
        ((255u8).to_variant(), "255"),
        ((0u16).to_variant(), "0"),
        ((65535u16).to_variant(), "65535"),
        ((0u32).to_variant(), "0"),
        ((4294967295u32).to_variant(), "4294967295"),
        //
        // signed
        ((127i8).to_variant(), "127"),
        ((-128i8).to_variant(), "-128"),
        ((32767i16).to_variant(), "32767"),
        ((-32768i16).to_variant(), "-32768"),
        ((2147483647i32).to_variant(), "2147483647"),
        ((-2147483648i32).to_variant(), "-2147483648"),
        ((9223372036854775807i64).to_variant(), "9223372036854775807"),
        (
            (-9223372036854775808i64).to_variant(),
            "-9223372036854775808",
        ),
    ];

    for (variant, string) in cases {
        assert_eq!(&variant.to_string(), string);
    }
}

#[itest]
fn variant_sys_conversion() {
    let v = Variant::from(7);
    let ptr = v.sys();

    let v2 = unsafe { Variant::new_from_sys(ptr) };
    assert_eq!(v2, v);
}

#[itest(skip)]
fn variant_sys_conversion2() {
    use godot::sys;

    // FIXME alignment, maybe use alloc()
    let mut buffer = [0u8; 50];

    let v = Variant::from(7);
    unsafe {
        v.clone().move_return_ptr(
            buffer.as_mut_ptr() as sys::GDExtensionTypePtr,
            sys::PtrcallType::Standard,
        )
    };

    let v2 = unsafe {
        Variant::new_with_uninit(|ptr| {
            std::ptr::copy(
                buffer.as_ptr(),
                ptr as *mut u8,
                std::mem::size_of_val(&*ptr),
            )
        })
    };
    assert_eq!(v2, v);
}

#[itest]
fn variant_null_object_is_nil() {
    use godot::sys;

    let mut node = Node2D::new_alloc();
    let node_path = NodePath::from("res://NonExisting.tscn");

    // Simulates an object that is returned but null
    // Use reflection to get a variant as return type
    let variant = node.call("get_node_or_null", vslice![node_path]);
    let raw_type: sys::GDExtensionVariantType =
        unsafe { sys::interface_fn!(variant_get_type)(variant.var_sys()) };

    // Verify that this appears as NIL to the user, even though it's internally OBJECT with a null object pointer
    assert_eq!(raw_type, sys::GDEXTENSION_VARIANT_TYPE_OBJECT);
    assert_eq!(variant.get_type(), VariantType::NIL);

    node.free();
}

#[itest]
fn variant_stringify() {
    assert_eq!("value".to_variant().stringify(), gstr("value"));
    assert_eq!(Variant::nil().stringify(), gstr("<null>"));
    assert_eq!(true.to_variant().stringify(), gstr("true"));
    assert_eq!(30.to_variant().stringify(), gstr("30"));
    assert_eq!(
        varray![1, "hello", false].to_variant().stringify(),
        gstr("[1, \"hello\", false]")
    );
    assert_eq!(
        vdict! { "KEY": 50 }.to_variant().stringify(),
        gstr("{ \"KEY\": 50 }")
    );
}

#[itest]
fn variant_booleanize() {
    assert!(gstr("string").to_variant().booleanize());
    assert!(10.to_variant().booleanize());
    assert!(varray![""].to_variant().booleanize());
    assert!(vdict! { "Key": 50 }.to_variant().booleanize());

    assert!(!Dictionary::new().to_variant().booleanize());
    assert!(!varray![].to_variant().booleanize());
    assert!(!0.to_variant().booleanize());
    assert!(!Variant::nil().booleanize());
    assert!(!gstr("").to_variant().booleanize());
}

#[itest]
fn variant_hash() {
    let hash_is_not_0 = [
        vdict! {}.to_variant(),
        gstr("").to_variant(),
        varray![].to_variant(),
    ];
    let self_equal = [
        gstr("string").to_variant(),
        varray![false, true, 4, "7"].to_variant(),
        0.to_variant(),
        vdict! { 0 : vdict!{ 0: 1 } }.to_variant(),
    ];

    for variant in hash_is_not_0 {
        assert_ne!(variant.hash_u32(), 0)
    }
    for variant in self_equal {
        assert_eq!(variant.hash_u32(), variant.hash_u32())
    }

    assert_eq!(Variant::nil().hash_u32(), 0);

    // It's not guaranteed that different object will have different hash, but it is
    // extremely unlikely for a collision to happen.
    assert_ne!(vdict! { 0: vdict! { 0: 0 } }, vdict! { 0: vdict! { 0: 1 } });
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

fn convert_relaxed_to<T, U>(from: T, expected_to: U)
where
    T: ToGodot + fmt::Debug,
    U: FromGodot + PartialEq + fmt::Debug,
{
    let variant = from.to_variant();
    let result = variant.try_to_relaxed::<U>();

    match result {
        Ok(to) => {
            assert_eq!(
                to, expected_to,
                "converting {from:?} to {to:?} resulted in unexpected value"
            );
        }
        Err(err) => {
            panic!("Conversion from {from:?} to {expected_to:?} failed: {err}");
        }
    }
}

fn convert_relaxed_fail<U>(from: impl ToGodot + fmt::Debug)
where
    U: FromGodot + PartialEq + fmt::Debug,
{
    let variant = from.to_variant();
    let result = variant.try_to_relaxed::<U>();

    match result {
        Ok(to) => {
            let to_type = godot::sys::short_type_name::<U>();
            panic!(
                "Conversion from {from:?} to {to_type:?} unexpectedly succeeded with value: {to:?}"
            );
        }
        Err(_err) => {}
    }
}

fn truncate_bad<T>(original_value: i64)
where
    T: FromGodot + Display,
{
    let variant = original_value.to_variant();
    let result = T::try_from_variant(&variant);

    if let Ok(back) = result {
        panic!(
            "{}::try_from_variant({}) should fail, but resulted in {}",
            std::any::type_name::<T>(),
            variant,
            back
        );
    }
}

fn equal<T, U>(lhs: T, rhs: U, expected: bool)
where
    T: ToGodot,
    U: ToGodot,
{
    if expected {
        assert_eq!(lhs.to_variant(), rhs.to_variant());
    } else {
        assert_ne!(lhs.to_variant(), rhs.to_variant());
    }
}

fn evaluate<T, U, E>(op: VariantOperator, lhs: T, rhs: U, expected: E)
where
    T: ToGodot,
    U: ToGodot,
    E: ToGodot,
{
    let lhs = lhs.to_variant();
    let rhs = rhs.to_variant();
    let expected = expected.to_variant();

    assert_eq!(lhs.evaluate(&rhs, op), Some(expected));
}

fn evaluate_fail<T, U>(op: VariantOperator, lhs: T, rhs: U)
where
    T: ToGodot,
    U: ToGodot,
{
    let lhs = lhs.to_variant();
    let rhs = rhs.to_variant();

    assert_eq!(lhs.evaluate(&rhs, op), None);
}

fn total_order<T, U>(lhs: T, rhs: U, expected_order: Ordering)
where
    T: ToGodot,
    U: ToGodot,
{
    fn eval(v: Option<Variant>) -> bool {
        v.expect("comparison is valid").to::<bool>()
    }

    let lhs = lhs.to_variant();
    let rhs = rhs.to_variant();

    let eq = eval(Variant::evaluate(&lhs, &rhs, VariantOperator::EQUAL));
    let ne = eval(Variant::evaluate(&lhs, &rhs, VariantOperator::NOT_EQUAL));
    let lt = eval(Variant::evaluate(&lhs, &rhs, VariantOperator::LESS));
    let le = eval(Variant::evaluate(&lhs, &rhs, VariantOperator::LESS_EQUAL));
    let gt = eval(Variant::evaluate(&lhs, &rhs, VariantOperator::GREATER));
    let ge = eval(Variant::evaluate(
        &lhs,
        &rhs,
        VariantOperator::GREATER_EQUAL,
    ));

    let true_rels;
    let false_rels;

    match expected_order {
        Ordering::Less => {
            true_rels = [ne, lt, le];
            false_rels = [eq, gt, ge];
        }
        Ordering::Equal => {
            true_rels = [eq, le, ge];
            false_rels = [ne, lt, gt];
        }
        Ordering::Greater => {
            true_rels = [ne, gt, ge];
            false_rels = [eq, lt, le];
        }
    }

    for rel in true_rels {
        assert!(
            rel,
            "total_order(rel=true, lhs={lhs:?}, rhs={rhs:?}, exp={expected_order:?})",
        );
    }
    for rel in false_rels {
        assert!(
            !rel,
            "total_order(rel=false, lhs={lhs:?}, rhs={rhs:?}, exp={expected_order:?})",
        );
    }
}

fn gstr(s: &str) -> GString {
    GString::from(s)
}

fn sname(s: &str) -> StringName {
    StringName::from(s)
}
