/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

/// Error caused by a reference-counted [`Gd`] instance which is not unique.
///
/// See [`Gd::try_to_unique()`] for futher information.
#[derive(Debug)]
pub enum NotUniqueError {
    /// The instance is shared and has a reference count greater than 1.
    Shared { ref_count: usize },

    /// The instance is not reference-counted; thus can't determine its uniqueness.
    NotRefCounted,
}

impl std::error::Error for NotUniqueError {}

impl std::fmt::Display for NotUniqueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Shared { ref_count } => {
                write!(f, "pointer is not unique, reference count: {ref_count}")
            }
            Self::NotRefCounted => {
                write!(f, "pointer is not reference-counted")
            }
        }
    }
}
