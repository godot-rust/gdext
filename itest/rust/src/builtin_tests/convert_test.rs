/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::{
    array, vdict, Array, Dictionary, GString, NodePath, StringName, Variant, VariantArray, Vector2,
    Vector2Axis,
};
use godot::classes::{Node, Resource};
use godot::meta;
use godot::meta::error::ConvertError;
use godot::meta::{AsArg, CowArg, FromGodot, GodotConvert, ToGodot};
use godot::obj::{Gd, NewAlloc};

use crate::framework::itest;

/// Ensure conversions we define have an associated value, and no underlying rust cause.
#[itest]
fn error_has_value_and_no_cause() {
    let node = Node::new_alloc();
    let errors: Vec<(ConvertError, &'static str)> = vec![
        (
            Variant::nil().try_to::<i64>().unwrap_err(),
            "`nil` -> `i64`",
        ),
        (
            VariantArray::new()
                .to_variant()
                .try_to::<GString>()
                .unwrap_err(),
            "`VariantArray` -> `GString`",
        ),
        (
            VariantArray::new()
                .to_variant()
                .try_to::<Array<i64>>()
                .unwrap_err(),
            "`VariantArray` -> `Array<i64>`",
        ),
        (
            Array::<Gd<Node>>::new()
                .to_variant()
                .try_to::<Array<Gd<Resource>>>()
                .unwrap_err(),
            "`Array<Gd<Node>>` -> `Array<Gd<Resource>>`",
        ),
        (
            node.clone().to_variant().try_to::<f32>().unwrap_err(),
            "`Gd<Node>` -> `f32`",
        ),
        (
            node.clone()
                .to_variant()
                .try_to::<Gd<Resource>>()
                .unwrap_err(),
            "`Gd<Node>` -> `Gd<Resource>`",
        ),
    ];

    for (err, err_str) in errors.into_iter() {
        assert!(
            err.value().is_some(),
            "{err_str} conversion has no value: {err:?}"
        );
        assert!(
            err.cause().is_none(),
            "{err_str} conversion should have no rust cause: {err:?}"
        );
    }

    node.free();
}

/// Check that the value stored in an error is the same as the value we tried to convert.
#[itest]
fn error_maintains_value() {
    let value = i32::MAX;
    let err = Vector2Axis::try_from_godot(value).unwrap_err();
    assert_eq!(format!("{value:?}"), format!("{:?}", err.value().unwrap()));

    let value = i64::MAX;
    let err = value.to_variant().try_to::<i32>().unwrap_err();
    assert_eq!(format!("{value:?}"), format!("{:?}", err.value().unwrap()));

    let value = f64::MAX.to_variant();
    let err = value.try_to::<i32>().unwrap_err();
    assert_eq!(format!("{value:?}"), format!("{:?}", err.value().unwrap()));
}

// Manual implementation of `GodotConvert` and related traits to ensure conversion works.
#[derive(PartialEq, Debug)]
struct ConvertedStruct {
    a: i32,
    b: f32,
}

impl ConvertedStruct {
    const MISSING_KEY_A: &'static str = "missing `a` key";
    const MISSING_KEY_B: &'static str = "missing `b` key";
    const TOO_MANY_KEYS: &'static str = "too many keys provided";
}

impl GodotConvert for ConvertedStruct {
    type Via = Dictionary;
}

impl ToGodot for ConvertedStruct {
    type Pass = godot::meta::ByValue;

    fn to_godot(&self) -> Self::Via {
        vdict! {
            "a": self.a,
            "b": self.b,
        }
    }
}

impl FromGodot for ConvertedStruct {
    fn try_from_godot(via: Self::Via) -> Result<Self, ConvertError> {
        let a = match via.get("a") {
            Some(a) => a,
            None => return Err(ConvertError::with_error_value(Self::MISSING_KEY_A, via)),
        };

        let b = match via.get("b") {
            Some(b) => b,
            None => return Err(ConvertError::with_error_value(Self::MISSING_KEY_B, via)),
        };

        if via.len() > 2 {
            return Err(ConvertError::with_error_value(Self::TOO_MANY_KEYS, via));
        }

        Ok(Self {
            a: a.try_to()?,
            b: b.try_to()?,
        })
    }
}

#[itest]
fn custom_convert_roundtrip() {
    let m = ConvertedStruct { a: 10, b: 12.34 };

    let as_dict = m.to_godot();
    assert_eq!(as_dict.get("a"), Some(m.a.to_variant()));
    assert_eq!(as_dict.get("b"), Some(m.b.to_variant()));

    let n = as_dict.to_variant().to::<ConvertedStruct>();
    assert_eq!(m, n, "from_variant");

    let o = ConvertedStruct::from_godot(as_dict);
    assert_eq!(m, o, "from_godot");
}

// Ensure all failure states for the `FromGodot` conversion of `ManuallyConverted` are propagated through the `try_to`
// method of `Variant` as they should be.
#[itest]
fn custom_convert_error_from_variant() {
    let missing_a = vdict! {
        "b": -0.001
    };
    let err = missing_a
        .to_variant()
        .try_to::<ConvertedStruct>()
        .expect_err("should be missing key `a`");

    assert_eq!(
        err.cause().unwrap().to_string(),
        ConvertedStruct::MISSING_KEY_A
    );

    let missing_b = vdict! {
        "a": 58,
    };
    let err = missing_b
        .to_variant()
        .try_to::<ConvertedStruct>()
        .expect_err("should be missing key `b`");

    assert_eq!(
        err.cause().unwrap().to_string(),
        ConvertedStruct::MISSING_KEY_B
    );

    let too_many_keys = vdict! {
        "a": 12,
        "b": 777.777,
        "c": "bar"
    };
    let err = too_many_keys
        .to_variant()
        .try_to::<ConvertedStruct>()
        .expect_err("should have too many keys");

    assert_eq!(
        err.cause().unwrap().to_string(),
        ConvertedStruct::TOO_MANY_KEYS
    );

    let wrong_type_a = vdict! {
        "a": "hello",
        "b": 28.41,
    };
    let err = wrong_type_a
        .to_variant()
        .try_to::<ConvertedStruct>()
        .expect_err("should have wrongly typed key `a`");

    assert!(err.cause().is_none());
    assert_eq!(
        format!("{:?}", err.value().unwrap()),
        format!("{:?}", "hello".to_variant())
    );

    let wrong_type_b = vdict! {
        "a": 29,
        "b": Vector2::new(1.0, 23.4),
    };
    let err = wrong_type_b
        .to_variant()
        .try_to::<ConvertedStruct>()
        .expect_err("should have wrongly typed key `b`");

    assert!(err.cause().is_none());
    assert_eq!(
        format!("{:?}", err.value().unwrap()),
        format!("{:?}", Vector2::new(1.0, 23.4).to_variant())
    );

    let too_big_value = vdict! {
        "a": i64::MAX,
        "b": f32::NAN
    };
    let err = too_big_value
        .to_variant()
        .try_to::<ConvertedStruct>()
        .expect_err("should have too big value for field `a`");

    assert!(err.cause().is_none());
    assert_eq!(
        format!("{:?}", err.value().unwrap()),
        format!("{:?}", i64::MAX)
    );
}

#[itest]
fn vec_to_array() {
    let from = vec![1, 2, 3];
    let to = from.to_variant().to::<Array<i32>>();
    assert_eq!(to, array![1, 2, 3]);

    let from = vec![GString::from("Hello"), GString::from("World")];
    let to = from.to_variant().to::<Array<GString>>();
    assert_eq!(to, array!["Hello", "World"]);

    // Invalid conversion.
    let from = vec![1, 2, 3];
    let to = from.to_variant().try_to::<Array<f32>>();
    assert!(to.is_err());
}

#[itest]
fn array_to_vec() {
    let from = array![1, 2, 3];
    let to = from.to_variant().to::<Vec<i32>>();
    assert_eq!(to, vec![1, 2, 3]);

    let from: Array<GString> = array!["Hello", "World"];
    let to = from.to_variant().to::<Vec<GString>>();
    assert_eq!(to, vec![GString::from("Hello"), GString::from("World")]);

    // Invalid conversion.
    let from = array![1, 2, 3];
    let to = from.to_variant().try_to::<Vec<f32>>();
    assert!(to.is_err());
}

#[itest]
fn rust_array_to_array() {
    let from = [1, 2, 3];
    let to = from.to_variant().to::<Array<i32>>();
    assert_eq!(to, array![1, 2, 3]);

    let from = [GString::from("Hello"), GString::from("World")];
    let to = from.to_variant().to::<Array<GString>>();
    assert_eq!(to, array!["Hello", "World"]);

    // Invalid conversion.
    let from = [1, 2, 3];
    let to = from.to_variant().try_to::<Array<f32>>();
    assert!(to.is_err());
}

#[itest]
fn array_to_rust_array() {
    let from = array![1, 2, 3];
    let to = from.to_variant().to::<[i32; 3]>();
    assert_eq!(to, [1, 2, 3]);

    let from: Array<GString> = array!["Hello", "World"];
    let to = from.to_variant().to::<[GString; 2]>();
    assert_eq!(to, [GString::from("Hello"), GString::from("World")]);

    // Invalid conversion.
    let from = array![1, 2, 3];
    let to = from.to_variant().try_to::<[f32; 3]>();
    assert!(to.is_err());
}

#[itest]
fn slice_to_array() {
    let from = &[1, 2, 3];
    let to = from.to_variant().to::<Array<i32>>();
    assert_eq!(to, array![1, 2, 3]);

    let from = &[GString::from("Hello"), GString::from("World")];
    let to = from.to_variant().to::<Array<GString>>();
    assert_eq!(to, array!["Hello", "World"]);

    // Invalid conversion.
    let from = &[1, 2, 3];
    let to = from.to_variant().try_to::<Array<f32>>();
    assert!(to.is_err());
}

fn as_gstr_arg<'a, T: 'a + AsArg<GString>>(t: T) -> CowArg<'a, GString> {
    t.into_arg()
}

fn as_sname_arg<'a, T: 'a + AsArg<StringName>>(t: T) -> CowArg<'a, StringName> {
    t.into_arg()
}

fn as_npath_arg<'a, T: 'a + AsArg<NodePath>>(t: T) -> CowArg<'a, NodePath> {
    t.into_arg()
}

#[itest]
fn strings_as_arg() {
    // Note: CowArg is an internal type.

    let str = "GodotRocks";
    let cstr = c"GodotRocks";
    let gstring = GString::from("GodotRocks");
    let sname = StringName::from("GodotRocks");
    let npath = NodePath::from("GodotRocks");

    assert_eq!(as_gstr_arg(str), CowArg::Owned(gstring.clone()));
    assert_eq!(as_gstr_arg(&gstring), CowArg::Borrowed(&gstring));
    assert_eq!(as_gstr_arg(sname.arg()), CowArg::Owned(gstring.clone()));
    assert_eq!(as_gstr_arg(npath.arg()), CowArg::Owned(gstring.clone()));

    assert_eq!(as_sname_arg(str), CowArg::Owned(sname.clone()));
    #[cfg(since_api = "4.2")]
    assert_eq!(as_sname_arg(cstr), CowArg::Owned(sname.clone()));
    assert_eq!(as_sname_arg(&sname), CowArg::Borrowed(&sname));
    assert_eq!(as_sname_arg(gstring.arg()), CowArg::Owned(sname.clone()));
    assert_eq!(as_sname_arg(npath.arg()), CowArg::Owned(sname.clone()));

    assert_eq!(as_npath_arg(str), CowArg::Owned(npath.clone()));
    assert_eq!(as_npath_arg(&npath), CowArg::Borrowed(&npath));
    assert_eq!(as_npath_arg(gstring.arg()), CowArg::Owned(npath.clone()));
    assert_eq!(as_npath_arg(sname.arg()), CowArg::Owned(npath.clone()));
}

#[itest]
fn to_arg_helpers() {
    let i: i8 = 3;
    let mut ints = array![1, 2];
    ints.push(meta::ref_to_arg(&i));
    ints.push(meta::owned_into_arg(i));

    assert_eq!(ints, array![1, 2, 3, 3]);

    let s = StringName::from("Godot");
    let mut names = array![&StringName::from("Hello")];
    names.push(meta::ref_to_arg(&s));
    names.push(meta::owned_into_arg(s));

    assert_eq!(names, array!["Hello", "Godot", "Godot"]);
}
