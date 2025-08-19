/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::obj::WeakGd;
use godot::prelude::*;

use crate::framework::{expect_panic, itest};

#[derive(GodotClass, Debug)]
#[class(base=RefCounted)]
struct RefcBasedDrop {
    pub base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for RefcBasedDrop {
    fn init(base: Base<RefCounted>) -> Self {
        let mut obj = base.to_weak_gd();
        obj.set_meta("meta", &"inited".to_variant());
        assert_eq!(obj.get_reference_count(), 1);
        Self { base }
    }
}

impl Drop for RefcBasedDrop {
    fn drop(&mut self) {
        let obj = self.to_weak_gd();
        assert_eq!(obj.get_meta("meta"), "inited".to_variant());

        // FIXME: Accessing godot methods except Object is UB.
        // assert_eq!(obj.get_reference_count(), 0);
    }
}

#[itest]
fn weak_gd_init_drop_refcounted() {
    let obj = RefcBasedDrop::new_gd();
    let weak = WeakGd::from_gd(&obj);
    drop(obj);
    expect_panic(
        "WeakGd calling Godot method with dead object should panic",
        || {
            weak.get_reference_count();
        },
    );
}
