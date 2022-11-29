/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#![allow(dead_code)]

use std::fmt::Debug;
use godot::bind::{godot_api, GodotClass, GodotExt};
use godot::engine::Node;
use godot::obj::{Base, Gd};
use godot::test::itest;
use std::marker::PhantomData;

/// A simple abstractio to see if we can derive GodotClass for Generic Structs
trait Abstraction {}
#[derive(Debug)]
struct A {}
#[derive(Debug)]
struct B {}
impl Abstraction for A {}
impl Abstraction for B {}


#[derive(GodotClass, Debug)]
#[class(init, base=Node)]
struct GenericStructTest<T> where T: Abstraction + Debug {
    #[base]
    some_base: Base<Node>,
    // Use phantom data so we're _only_ testing the generic aspect
    phantom_data: PhantomData<T>
}

#[godot_api]
impl<T> GenericStructTest<T> where T: Abstraction + Debug {
    fn get_phantom_data(&self) -> String {
        format!("{:?}", self.phantom_data)
    }
}

#[godot_api]
impl<T> GodotExt for GenericStructTest<T> where T: Abstraction + Debug {}

pub(crate) fn run() -> bool {
    let mut ok = true;
    ok &= test_to_string();
    ok
}

// pub(crate) fn register() {
//     godot::register_class::<VirtualMethodTest>();
// }

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[itest]
fn test_to_string() {
    let _obj1 = Gd::<GenericStructTest<A>>::new_default();
    dbg!(_obj1);
    let _obj2 = Gd::<GenericStructTest<B>>::new_default();
    dbg!(_obj2);
}
