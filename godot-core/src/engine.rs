/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#[deprecated = "Classes have been moved to `godot::classes`."]
pub use crate::classes::*;

#[deprecated = "Enums have been moved to `godot::global`."]
pub mod global {
    pub use crate::builtin::{Corner, EulerOrder, Side};
    pub use crate::global::*;
}

#[deprecated = "Utility functions have been moved to `godot::global`."]
pub mod utilities {
    pub use crate::global::*;
}

#[deprecated = "Native structures have been moved to `godot::classes::native`."]
pub mod native {
    pub use crate::gen::native::*;
}

#[deprecated = "`godot::classes::translate` has been moved to `godot::tools`."]
pub mod translate {
    pub use crate::tools::{tr, tr_n};
}

#[deprecated = "`create_script_instance` has been moved to `godot::obj::script`."]
pub use crate::obj::script::create_script_instance;

#[deprecated = "`ScriptInstance` has been moved to `godot::obj::script`."]
pub use crate::obj::script::ScriptInstance;

#[deprecated = "`SiMut` has been moved to `godot::obj::script`."]
pub use crate::obj::script::SiMut;

#[deprecated = "`GFile` has been moved to `godot::tools`."]
pub use crate::tools::GFile;

#[deprecated = "`IoError` has been moved to `godot::meta::error`."]
pub use crate::meta::error::IoError;

#[deprecated = "`save` has been moved to `godot::global`."]
pub use crate::tools::save;

#[deprecated = "`try_save` has been moved to `godot::global`."]
pub use crate::tools::try_save;

#[deprecated = "`load` has been moved to `godot::global`."]
pub use crate::tools::load;

#[deprecated = "`try_load` has been moved to `godot::global`."]
pub use crate::tools::try_load;
