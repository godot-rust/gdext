/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#![allow(dead_code)]

#[rustfmt::skip]
#[path = "gen/gen_ffi.rs"]
mod gen_ffi;

pub(crate) fn run() -> bool {
    true
}
