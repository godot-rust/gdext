/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt::Debug;

use crate::itest;
use godot::bind::FromVariant;
use godot::bind::ToVariant;
use godot::builtin::{dict, varray, FromVariant, ToVariant};

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

fn roundtrip<T, U>(value: T, expected: U)
where
    T: ToVariant + FromVariant + std::cmp::PartialEq + Debug,
    U: ToVariant,
{
    let expected = expected.to_variant();

    assert_eq!(value.to_variant(), expected, "testing converting to");
    assert_eq!(
        value,
        T::from_variant(&expected),
        "testing converting back from"
    );
}

#[itest]
fn unit_struct() {
    roundtrip(
        StructUnit,
        dict! { "StructUnit": godot::builtin::Variant::nil() },
    );
}

#[itest]
fn new_type_struct() {
    roundtrip(
        StructNewType(String::from("five")),
        dict! { "StructNewType" : "five" },
    )
}

#[itest]
fn tuple_struct() {
    roundtrip(
        StructTuple(String::from("one"), 2),
        dict! {
            "StructTuple": varray!["one", 2]
        },
    )
}

#[itest]
fn named_struct() {
    roundtrip(
        StructNamed {
            field1: String::from("four"),
            field2: 5,
        },
        dict! {
            "StructNamed": dict! { "field1": "four", "field2": 5 }
        },
    )
}

#[itest]
fn generics() {
    roundtrip(
        StructGenWhere(String::from("4")),
        dict! { "StructGenWhere": "4" },
    )
}

impl Bound for String {}

#[itest]
fn generics_bound() {
    roundtrip(
        StructGenBound(String::from("4")),
        dict! { "StructGenBound": "4" },
    )
}

#[itest]
fn enum_unit() {
    roundtrip(Enum::Unit, dict! { "Enum": "Unit" })
}

#[itest]
fn enum_one_tuple() {
    roundtrip(
        Enum::OneTuple(4),
        dict! {
            "Enum": dict! { "OneTuple" : 4 }
        },
    )
}

#[itest]
fn enum_tuple() {
    roundtrip(
        Enum::Tuple(String::from("four"), 5),
        dict! { "Enum": dict! { "Tuple" : varray!["four", 5] } },
    )
}

#[itest]
fn enum_named() {
    roundtrip(
        Enum::Named {
            data: String::from("data"),
        },
        dict! {
            "Enum": dict!{ "Named": dict!{ "data": "data" } }
        },
    )
}
