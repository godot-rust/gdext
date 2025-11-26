/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt::Debug;

use godot::builtin::{GString, Vector2};
use godot::meta::ToGodot;
use godot::register::GodotConvert;

use crate::common::roundtrip;
use crate::framework::itest;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// General FromGodot/ToGodot derive tests

#[derive(GodotConvert, PartialEq, Debug)]
#[godot(transparent)]
struct TupleNewtype(GString);

#[derive(GodotConvert, PartialEq, Debug)]
#[godot(transparent)]
struct NamedNewtype {
    field1: Vector2,
}

#[derive(GodotConvert, Clone, PartialEq, Debug)]
#[godot(via = GString)]
enum EnumStringy {
    A,
    B = (1 + 2),
    C = 10,
    D = 50,
    E,
    F = (EnumInty::B as isize),
}

#[derive(GodotConvert, Clone, PartialEq, Debug)]
#[godot(via = i64)]
enum EnumInty {
    A = 10,
    B,
    C,
    D = 1,
    E,
}

#[derive(GodotConvert, Clone, PartialEq, Debug)]
#[godot(via = i64)]
enum EnumIntyWithExprs {
    G = (1 + 2),
    H,
    I = (EnumInty::B as isize),
}

#[itest]
fn newtype_tuple_struct() {
    roundtrip(TupleNewtype(GString::from("hello!")));
}

#[itest]
fn newtype_named_struct() {
    roundtrip(NamedNewtype {
        field1: Vector2::new(10.0, 25.0),
    });
}

#[itest]
fn enum_stringy() {
    roundtrip(EnumStringy::A);
    roundtrip(EnumStringy::B);
    roundtrip(EnumStringy::C);
    roundtrip(EnumStringy::D);
    roundtrip(EnumStringy::E);
    roundtrip(EnumStringy::F);

    assert_eq!(EnumStringy::A.to_godot(), "A");
    assert_eq!(EnumStringy::B.to_godot(), "B");
    assert_eq!(EnumStringy::C.to_godot(), "C");
    assert_eq!(EnumStringy::D.to_godot(), "D");
    assert_eq!(EnumStringy::E.to_godot(), "E");
    assert_eq!(EnumStringy::F.to_godot(), "F");

    // Rust-side discriminants.
    assert_eq!(EnumStringy::A as isize, 0);
    assert_eq!(EnumStringy::B as isize, 3);
    assert_eq!(EnumStringy::C as isize, 10);
    assert_eq!(EnumStringy::D as isize, 50);
    assert_eq!(EnumStringy::E as isize, 51);
    assert_eq!(EnumStringy::F as isize, 11);
}

#[itest]
fn enum_inty() {
    roundtrip(EnumInty::A);
    roundtrip(EnumInty::B);
    roundtrip(EnumInty::C);
    roundtrip(EnumInty::D);
    roundtrip(EnumInty::E);

    assert_eq!(EnumInty::A.to_godot(), 10);
    assert_eq!(EnumInty::B.to_godot(), 11);
    assert_eq!(EnumInty::C.to_godot(), 12);
    assert_eq!(EnumInty::D.to_godot(), 1);
    assert_eq!(EnumInty::E.to_godot(), 2);
}

#[itest]
fn enum_inty_with_complex_exprs() {
    roundtrip(EnumIntyWithExprs::G);
    roundtrip(EnumIntyWithExprs::H);
    roundtrip(EnumIntyWithExprs::I);

    assert_eq!(EnumIntyWithExprs::G.to_godot(), 3);
    assert_eq!(EnumIntyWithExprs::H.to_godot(), 4);
    assert_eq!(EnumIntyWithExprs::I.to_godot(), 11);

    // Rust-side discriminants.
    assert_eq!(EnumIntyWithExprs::G as isize, 3);
    assert_eq!(EnumIntyWithExprs::H as isize, 4);
    assert_eq!(EnumIntyWithExprs::I as isize, 11);
}

macro_rules! test_inty {
    ($T:ident, $test_name:ident, $class_name:ident) => {
        #[derive(GodotConvert, Clone, PartialEq, Debug)]
        #[godot(via = $T)]
        enum $class_name {
            A,
            B,
        }

        #[itest]
        fn $test_name() {
            roundtrip($class_name::A);
            roundtrip($class_name::B);
        }
    };
}

test_inty!(i8, test_enum_i8, EnumI8);
test_inty!(i16, test_enum_16, EnumI16);
test_inty!(i32, test_enum_i32, EnumI32);
test_inty!(i64, test_enum_i64, EnumI64);
test_inty!(u8, test_enum_u8, EnumU8);
test_inty!(u16, test_enum_u16, EnumU16);
test_inty!(u32, test_enum_u32, EnumU32);
