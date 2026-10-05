/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicI32, Ordering};

use godot::prelude::*;

use crate::framework::itest;

static SUCCESSFUL_CALLS: AtomicI32 = AtomicI32::new(0);

#[derive(GodotClass)]
#[class(init)]
struct ConversionTest {}

#[godot_api]
impl ConversionTest {
    #[func]
    fn accept_i32(value: i32) -> String {
        SUCCESSFUL_CALLS.fetch_add(1, Ordering::SeqCst);
        value.to_string()
    }

    #[func]
    fn accept_f32(value: f32) -> String {
        SUCCESSFUL_CALLS.fetch_add(1, Ordering::SeqCst);
        value.to_string()
    }

    #[func]
    fn return_i32() -> i32 {
        SUCCESSFUL_CALLS.fetch_add(1, Ordering::SeqCst);
        123
    }

    #[func]
    fn return_f32() -> f32 {
        SUCCESSFUL_CALLS.fetch_add(1, Ordering::SeqCst);
        123.45
    }

    #[func]
    fn hash_to_btree(map: HashMap<u8, Vector2i>) -> BTreeMap<u8, Vector2i> {
        SUCCESSFUL_CALLS.fetch_add(1, Ordering::SeqCst);
        map.into_iter().collect()
    }

    #[func]
    fn btree_to_hash(map: BTreeMap<GString, Gd<RefCounted>>) -> HashMap<GString, Gd<RefCounted>> {
        SUCCESSFUL_CALLS.fetch_add(1, Ordering::SeqCst);
        map.into_iter().collect()
    }

    #[func]
    fn successful_calls() -> i32 {
        SUCCESSFUL_CALLS.load(Ordering::SeqCst)
    }
}

#[itest]
fn test_convert_untyped_dict() {
    let mut conv = ConversionTest::new_gd();

    let dict = vdict! { 1u8 => Vector2i::new(2, 3), 4u8 => Vector2i::new(5, 6) };
    let result = conv.call("hash_to_btree", vslice![dict]);
    assert_eq!(result.to::<VarDictionary>(), dict);

    let dict = vdict! { "one" => &RefCounted::new_gd(), "two" => &RefCounted::new_gd() };
    let result = conv.call("btree_to_hash", vslice![dict]);
    assert_eq!(result.to::<VarDictionary>(), dict);
}
