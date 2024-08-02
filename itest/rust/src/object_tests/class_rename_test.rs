/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::framework::itest;
use godot::prelude::*;

pub mod dont_rename {
    use super::*;

    #[derive(GodotClass)]
    #[class(no_init)]
    pub struct RepeatMe {}
}

pub mod rename {
    use super::*;

    #[derive(GodotClass)]
    #[class(rename=NoRepeat, no_init)]
    pub struct RepeatMe {}
}

#[itest]
fn renaming_changes_the_name() {
    assert_ne!(
        dont_rename::RepeatMe::class_name(),
        rename::RepeatMe::class_name()
    );
    assert_eq!(dont_rename::RepeatMe::class_name().to_string(), "RepeatMe");
    assert_eq!(rename::RepeatMe::class_name().to_string(), "NoRepeat");
}
