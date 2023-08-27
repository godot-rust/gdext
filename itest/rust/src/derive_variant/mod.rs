/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt::Debug;

use crate::framework::itest;
use crate::variant_test::roundtrip;
use godot::bind::FromVariant;
use godot::bind::ToVariant;
use godot::builtin::{dict, varray, FromVariant, ToVariant, Variant};

#[macro_export]
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
#[macro_use]
mod enums_skip;
//#[macro_use]
mod structs_skip;

#[derive(FromVariant, ToVariant, PartialEq, Debug)]
struct StructUnit;

#[derive(FromVariant, ToVariant, PartialEq, Debug)]
struct StructNewType(String);

#[derive(FromVariant, ToVariant, PartialEq, Debug)]
struct StructTuple(String, i32);

#[derive(FromVariant, ToVariant, PartialEq, Debug)]
struct StructNamed {
    field1: String,
    field2: i32,
}

#[derive(FromVariant, ToVariant, PartialEq, Debug)]
struct StructGenWhere<T>(T)
where
    T: ToVariant + FromVariant;

trait Bound {}

#[derive(FromVariant, ToVariant, PartialEq, Debug)]
struct StructGenBound<T: Bound + ToVariant + FromVariant>(T);

#[derive(FromVariant, ToVariant, PartialEq, Debug, Clone)]
enum Uninhabited {}

#[derive(FromVariant, ToVariant, PartialEq, Debug, Clone)]
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
