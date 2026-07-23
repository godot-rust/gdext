/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt::Debug;

use godot::builtin::{GString, Vector2, array, dict, vslice};
use godot::classes::RefCounted;
use godot::meta::{GodotConvert, ToGodot};
use godot::obj::{Base, NewGd};
use godot::register::{GodotClass, godot_api};

use crate::common::roundtrip;
use crate::framework::itest;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// General FromGodot/ToGodot derive tests

// Class exercising enums as `#[func]` parameters/returns and `#[signal]` parameters.
#[derive(GodotClass)]
#[class(init)]
struct EnumFuncs {
    _base: Base<RefCounted>,
    last_int: Option<EnumInty>,
    last_str: Option<EnumStringy>,
}

#[godot_api]
impl EnumFuncs {
    #[func]
    fn next_int(&self, i: EnumInty) -> EnumInty {
        match i {
            EnumInty::A => EnumInty::B,
            EnumInty::B => EnumInty::C,
            EnumInty::C => EnumInty::D,
            EnumInty::D => EnumInty::E,
            EnumInty::E => EnumInty::A,
        }
    }

    #[func]
    fn next_str(&self, s: EnumStringy) -> EnumStringy {
        match s {
            EnumStringy::A => EnumStringy::B,
            EnumStringy::B => EnumStringy::C,
            EnumStringy::C => EnumStringy::D,
            EnumStringy::D => EnumStringy::E,
            EnumStringy::E => EnumStringy::F,
            EnumStringy::F => EnumStringy::A,
        }
    }

    #[signal]
    fn take_enums(i: EnumInty, s: EnumStringy);
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Types under test

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

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Tests

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

#[itest]
fn enum_in_func() {
    let f = EnumFuncs::new_gd();

    f.signals().take_enums().connect_self(|this, i, s| {
        this.last_int = Some(this.next_int(i));
        this.last_str = Some(this.next_str(s));
    });

    assert_eq!(f.bind().last_int, None);
    assert_eq!(f.bind().last_str, None);

    f.signals().take_enums().emit(EnumInty::B, EnumStringy::F);

    assert_eq!(f.bind().last_int, Some(EnumInty::C));
    assert_eq!(f.bind().last_str, Some(EnumStringy::A));
}

#[itest]
fn enum_in_func_dynamic() {
    let mut f = EnumFuncs::new_gd();

    // Try with both the strongly-typed enum and the underlying int/string.
    let ia_next = f.call("next_int", vslice![EnumInty::A as i32]);
    let ib_next = f.call("next_int", vslice![EnumInty::B]);
    let sc_next = f.call("next_str", vslice!["C"]);
    let sd_next = f.call("next_str", vslice![EnumStringy::D]);

    assert_eq!(ia_next.to::<EnumInty>(), EnumInty::B);
    assert_eq!(ib_next.to::<EnumInty>(), EnumInty::C);
    assert_eq!(sc_next.to::<EnumStringy>(), EnumStringy::D);
    assert_eq!(sd_next.to::<EnumStringy>(), EnumStringy::E);
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

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Array and Dictionary element tests for GodotConvert enums.

#[itest]
fn enum_user_in_array() {
    use godot::builtin::Array;

    // String-based enum in array.
    let mut arr = Array::new();
    arr.push(EnumStringy::A);
    arr.push(EnumStringy::C);
    assert_eq!(arr.get(0), Some(EnumStringy::A));
    assert_eq!(arr.get(1), Some(EnumStringy::C));

    // Int-based enum in array.
    let arr = array![EnumInty::A, EnumInty::D];
    assert_eq!(arr.get(0), Some(EnumInty::A));
    assert_eq!(arr.get(1), Some(EnumInty::D));
}

#[itest]
fn enum_user_in_dictionary() {
    use godot::builtin::{Dictionary, Vector2i};

    // Enum as dictionary value.
    let mut dict = Dictionary::<Vector2i, EnumStringy>::new();
    dict.set(Vector2i::new(1, 2), EnumStringy::A);
    dict.set(Vector2i::new(3, 4), EnumStringy::C);
    assert_eq!(dict.at(Vector2i::new(1, 2)), EnumStringy::A);
    assert_eq!(dict.at(Vector2i::new(3, 4)), EnumStringy::C);

    // Enum as dictionary key.
    let dict: Dictionary<EnumInty, GString> = dict! {
        EnumInty::A => "first",
        EnumInty::B => "second",
    };
    assert_eq!(dict.at(EnumInty::A), GString::from("first"));
    assert_eq!(dict.at(EnumInty::B), GString::from("second"));
}

#[itest]
fn enum_engine_in_array() {
    use godot::builtin::{Array, Dictionary, Side};

    // Engine enum in array.
    let mut arr = Array::<Side>::new();
    arr.push(Side::LEFT);
    arr.push(Side::RIGHT);
    assert_eq!(arr.get(0), Some(Side::LEFT));
    assert_eq!(arr.get(1), Some(Side::RIGHT));

    // Engine enum as dictionary value.
    let mut dict = Dictionary::<i64, Side>::new();
    dict.set(0, Side::TOP);
    dict.set(1, Side::BOTTOM);
    assert_eq!(dict.at(0), Side::TOP);
    assert_eq!(dict.at(1), Side::BOTTOM);
}
