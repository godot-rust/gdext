/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::prelude::*;

use super::multiple_impl_blocks_test::MultipleImplBlocks;

#[godot_api(secondary)]
impl MultipleImplBlocks {
    #[func]
    fn third(&self) -> String {
        "3rd result".to_string()
    }
}
