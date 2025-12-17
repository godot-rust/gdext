/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Metadata registry for user-defined classes.
//!
//! This module provides a global registry for storing property and method names
//! of user-defined classes. Engine classes use compile-time metadata via the
//! `ClassMetadata` trait implementations.

use std::collections::{HashMap, HashSet};

use crate::meta::ClassId;
use crate::sys::Global;

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Metadata information for a single class.
#[derive(Default)]
pub struct ClassMetadataInfo {
    pub properties: HashSet<String>,
    pub functions: HashSet<String>,
}

/// Global registry mapping class IDs to their metadata.
static USER_CLASS_METADATA: Global<HashMap<ClassId, ClassMetadataInfo>> = Global::default();

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Registration functions

/// Register a property for a user-defined class.
///
/// This is called during class registration (from derive macros or builder API).
pub(crate) fn register_user_property(class_id: ClassId, name: &str) {
    let mut metadata = USER_CLASS_METADATA.lock();
    metadata
        .entry(class_id)
        .or_default()
        .properties
        .insert(name.to_string());
}

/// Register a function for a user-defined class.
///
/// This is called during class registration (from derive macros or builder API).
pub(crate) fn register_user_function(class_id: ClassId, name: &str) {
    let mut metadata = USER_CLASS_METADATA.lock();
    metadata
        .entry(class_id)
        .or_default()
        .functions
        .insert(name.to_string());
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Query functions

/// Check if a user-defined class has a property with the given name.
///
/// Returns `false` if the class is not registered or doesn't have the property.
pub(crate) fn has_user_property(class_id: ClassId, name: &str) -> bool {
    let metadata = USER_CLASS_METADATA.lock();
    metadata
        .get(&class_id)
        .map_or(false, |info| info.properties.contains(name))
}

/// Check if a user-defined class has a function with the given name.
///
/// Returns `false` if the class is not registered or doesn't have the function.
pub(crate) fn has_user_function(class_id: ClassId, name: &str) -> bool {
    let metadata = USER_CLASS_METADATA.lock();
    metadata
        .get(&class_id)
        .map_or(false, |info| info.functions.contains(name))
}
