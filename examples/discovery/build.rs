/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use hot_reload::godot_discovery::ExtensionDiscovery;
use hot_reload::HotReload;

use std::fs::File;
use std::io::Write;

fn main() -> Result<(), std::io::Error> {
    let mut f = File::create("discovered.txt")?;

    for c in HotReload::discover_classes() {
        writeln!(f, "Discovered: class {}, base {}", c.name(), c.base_class())?;
    }

    Ok(())
}
