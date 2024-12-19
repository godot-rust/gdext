/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::private::{ClassPlugin, PluginItem};

pub trait ExtensionDiscovery {
    fn discover_classes() -> Vec<DiscoveredClass>;
}

pub struct DiscoveredClass {
    name: String,
    base_class: String,
}

impl DiscoveredClass {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn base_class(&self) -> &str {
        &self.base_class
    }
}

#[doc(hidden)]
pub fn __discover() -> Vec<DiscoveredClass> {
    let mut classes = vec![];

    crate::private::iterate_plugins(|elem: &ClassPlugin| {
        let PluginItem::Struct {
            base_class_name, ..
        } = elem.item
        else {
            return;
        };

        let class = DiscoveredClass {
            name: elem.class_name.to_string(),
            base_class: base_class_name.to_string(),
        };

        classes.push(class);
    });

    classes
}
