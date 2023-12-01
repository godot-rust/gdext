/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt::Debug;

use godot::bind::{FromGodot, GodotConvert, ToGodot};
use godot::builtin::meta::{FromGodot, ToGodot};
use godot::builtin::{dict, varray, Variant};

use crate::common::roundtrip;
use crate::framework::itest;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// General FromGodot/ToGodot derive tests

#[derive(FromGodot, ToGodot, GodotConvert, PartialEq, Debug)]
struct StructUnit;

#[derive(FromGodot, ToGodot, GodotConvert, PartialEq, Debug)]
struct StructNewType(String);

#[derive(FromGodot, ToGodot, GodotConvert, PartialEq, Debug)]
struct StructTuple(String, i32);

#[derive(FromGodot, ToGodot, GodotConvert, PartialEq, Debug)]
struct StructNamed {
    field1: String,
    field2: i32,
}

#[derive(FromGodot, ToGodot, GodotConvert, PartialEq, Debug)]
struct StructGenWhere<T>(T)
where
    T: ToGodot + FromGodot;

trait Bound {}

#[derive(FromGodot, ToGodot, GodotConvert, PartialEq, Debug)]
struct StructGenBound<T: Bound + ToGodot + FromGodot>(T);

#[derive(FromGodot, ToGodot, GodotConvert, Clone, PartialEq, Debug)]
enum Uninhabited {}

#[derive(FromGodot, ToGodot, GodotConvert, Clone, PartialEq, Debug)]
enum Enum {
    Unit,
    OneTuple(i32),
    Named { data: String },
    Tuple(String, i32),
}

#[itest]
fn unit_struct() {
    roundtrip(StructUnit);
    roundtrip(dict! { "StructUnit": godot::builtin::Variant::nil() });
}

#[itest]
fn new_type_struct() {
    roundtrip(StructNewType(String::from("five")));
    roundtrip(dict! { "StructNewType" : "five" })
}

#[itest]
fn tuple_struct() {
    roundtrip(StructTuple(String::from("one"), 2));
    roundtrip(dict! {
        "StructTuple": varray!["one", 2]
    });
}

#[itest]
fn named_struct() {
    roundtrip(StructNamed {
        field1: String::from("four"),
        field2: 5,
    });
    roundtrip(dict! {
        "StructNamed": dict! { "field1": "four", "field2": 5 }
    });
}

#[itest]
fn generics() {
    roundtrip(StructGenWhere(String::from("4")));
    roundtrip(dict! { "StructGenWhere": "4" });
}

impl Bound for String {}

#[itest]
fn generics_bound() {
    roundtrip(StructGenBound(String::from("4")));
    roundtrip(dict! { "StructGenBound": "4" });
}

#[itest]
fn enum_unit() {
    roundtrip(Enum::Unit);
    roundtrip(dict! { "Enum": "Unit" });
}

#[itest]
fn enum_one_tuple() {
    roundtrip(Enum::OneTuple(4));
    roundtrip(dict! {
        "Enum": dict! { "OneTuple" : 4 }
    });
}

#[itest]
fn enum_tuple() {
    roundtrip(Enum::Tuple(String::from("four"), 5));
    roundtrip(dict! { "Enum": dict! { "Tuple" : varray!["four", 5] } });
}

#[itest]
fn enum_named() {
    roundtrip(Enum::Named {
        data: String::from("data"),
    });
    roundtrip(dict! {
        "Enum": dict!{ "Named": dict!{ "data": "data" } }
    });
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Skipping of enums

macro_rules! roundtrip_with_skip {
    ($name_to:ident, $name_from:ident, $value:expr, $to_var:expr, $from_var:expr) => {
        #[itest]
        fn $name_to() {
            let s = $value;
            assert_eq!(s.to_variant(), $to_var.to_variant(),)
        }

        #[itest]
        fn $name_from() {
            assert_eq!(EnumWithSkip::from_variant(&$to_var.to_variant()), $from_var);
        }
    };
}

#[derive(ToGodot, FromGodot, GodotConvert, Default, Clone, PartialEq, Debug)]
enum EnumWithSkip {
    #[variant(skip)]
    Skipped(String),
    NewType(#[variant(skip)] String),
    PartSkippedTuple(#[variant(skip)] String, String),
    PartSkippedNamed {
        #[variant(skip)]
        skipped_data: String,
        data: String,
    },
    #[default]
    Default,
}

roundtrip_with_skip!(
    skipped_to_variant,
    skipped_from_variant,
    EnumWithSkip::Skipped("one".to_string()),
    dict! { "EnumWithSkip" : Variant::nil() },
    EnumWithSkip::default()
);

roundtrip_with_skip!(
    skipped_newtype_to_variant,
    skipped_newtype_from_variant,
    EnumWithSkip::NewType("whatever".to_string()),
    dict! { "EnumWithSkip" : dict!{ "NewType" : Variant::nil() } },
    EnumWithSkip::NewType(String::default())
);

roundtrip_with_skip!(
    skipped_tuple_to_variant,
    skipped_tuple_from_variant,
    EnumWithSkip::PartSkippedTuple("skipped".to_string(), "three".to_string()),
    dict! {
        "EnumWithSkip": dict!{
            "PartSkippedTuple" : varray!["three"]
        }
    },
    EnumWithSkip::PartSkippedTuple(String::default(), "three".to_string())
);

roundtrip_with_skip!(
    named_skipped_to_variant,
    named_skipped_from_variant,
    EnumWithSkip::PartSkippedNamed {
        skipped_data: "four".to_string(),
        data: "five".to_string(),
    },
    dict! {
        "EnumWithSkip": dict!{
            "PartSkippedNamed" : dict! { "data" : "five" }
        }
    },
    EnumWithSkip::PartSkippedNamed {
        data: "five".to_string(),
        skipped_data: String::default()
    }
);

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Skipping of structs

#[derive(ToGodot, FromGodot, GodotConvert, Default, PartialEq, Debug)]
struct NewTypeStructWithSkip(#[variant(skip)] String);

#[derive(ToGodot, FromGodot, GodotConvert, Default, PartialEq, Debug)]
struct StructWithSkip {
    #[variant(skip)]
    skipped_field: String,
    field: String,
}

#[itest]
fn new_type_to_variant() {
    assert_eq!(
        NewTypeStructWithSkip("four".to_string()).to_variant(),
        dict! {"NewTypeStructWithSkip" : varray![] }.to_variant()
    );
}

#[itest]
fn new_type_from_variant() {
    let s = NewTypeStructWithSkip("four".to_string());
    assert_eq!(
        NewTypeStructWithSkip::from_variant(&s.to_variant()),
        NewTypeStructWithSkip::default()
    )
}

#[itest]
fn struct_with_skip_to_variant() {
    assert_eq!(
        StructWithSkip {
            skipped_field: "four".to_string(),
            field: "seven".to_string(),
        }
        .to_variant(),
        dict! { "StructWithSkip" : dict! { "field" : "seven" } }.to_variant()
    );
}

#[itest]
fn struct_with_skip_from_variant() {
    assert_eq!(
        StructWithSkip {
            field: "seven".to_string(),
            ..Default::default()
        },
        StructWithSkip::from_variant(
            &StructWithSkip {
                skipped_field: "four".to_string(),
                field: "seven".to_string(),
            }
            .to_variant()
        )
    );
}
