/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod any_array;
mod any_dictionary;
mod array;
mod array_functional_ops;
mod dictionary;
mod extend_buffer;
mod packed_array;
mod packed_array_element;

// Re-export in godot::builtin.
pub(crate) mod containers {
    pub use super::any_array::AnyArray;
    pub use super::any_dictionary::AnyDictionary;
    pub use super::array::{Array, VarArray};
    #[allow(deprecated)]
    pub use super::dictionary::Dictionary;
    pub use super::dictionary::VarDictionary;
    pub use super::packed_array::*;
}

// Re-export in godot::builtin::iter.
pub(crate) mod iterators {
    pub use super::any_array::AnyArrayIter;
    pub use super::any_dictionary::{AnyDictIter, AnyDictKeys, AnyDictValues};
    pub use super::array::ArrayIter;
    pub use super::array_functional_ops::ArrayFunctionalOps;
    pub use super::dictionary::{DictIter, DictKeys, DictValues};
}

// Re-export in godot::meta.
pub use packed_array_element::PackedElement;
