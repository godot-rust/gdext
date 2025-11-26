/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#![allow(dead_code)]

// While some of these lints are legitimate, no one cares for this generated code, and it's definitely not worth complicating the generator.
#[rustfmt::skip]
#[allow(clippy::partialeq_to_none)] // i == None  ->  i.is_none()
#[allow(clippy::cmp_owned)] // i == GString::from("hello")  ->  i == "hello"
pub mod gen_ffi {
    include!(concat!(env!("OUT_DIR"), "/gen_ffi.rs"));
}
