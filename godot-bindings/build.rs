/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// `api-*` validation is required here because otherwise, user gets confusing error about conflicting module imports.
// We can also not use this in dependent crates, e.g. in godot/build.rs, since this crate is compiled before and already causes the error.
// It's the only purpose of this build.rs file. If a better solution is found, this file can be removed.

#[rustfmt::skip]
fn main() {
    let mut count = 0;
    if cfg!(feature = "api-custom") { count += 1; }
    if cfg!(feature = "api-custom-json") { count += 1; }

    // [version-sync] [[
    //  [line] \tif cfg!(feature = "api-$kebabVersion") { count += 1; }
    if cfg!(feature = "api-4-2") { count += 1; }
    if cfg!(feature = "api-4-2-1") { count += 1; }
    if cfg!(feature = "api-4-2-2") { count += 1; }
    if cfg!(feature = "api-4-3") { count += 1; }
    if cfg!(feature = "api-4-4") { count += 1; }
    if cfg!(feature = "api-4-5") { count += 1; }
    // ]]

    assert!(count <= 1, "ERROR: at most one `api-*` feature can be enabled");
}
