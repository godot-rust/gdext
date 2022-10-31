/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::{ExtensionApi, RustTy};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub(crate) struct Context<'a> {
    engine_classes: HashSet<&'a str>,
    builtin_types: HashSet<&'a str>,
    singletons: HashSet<&'a str>,
    inheritance_tree: InheritanceTree,
    cached_rust_types: HashMap<String, RustTy>,
}

impl<'a> Context<'a> {
    pub fn build_from_api(api: &'a ExtensionApi) -> Self {
        let mut ctx = Context::default();

        for class in api.singletons.iter() {
            ctx.singletons.insert(class.name.as_str());
        }

        for builtin in api.builtin_classes.iter() {
            let ty_name = builtin.name.as_str();
            ctx.builtin_types.insert(ty_name);
        }

        for class in api.classes.iter() {
            let class_name = class.name.as_str();
            #[cfg(feature = "minimal")]
            if !crate::SELECTED_CLASSES.contains(&class_name) {
                continue;
            }

            println!("-- add engine class {}", class_name);
            ctx.engine_classes.insert(class_name);

            if let Some(base) = class.inherits.as_ref() {
                println!("  -- inherits {}", base);
                ctx.inheritance_tree
                    .insert(class_name.to_string(), base.clone());
            }
        }
        ctx
    }

    // pub fn is_engine_class(&self, class_name: &str) -> bool {
    //     self.engine_classes.contains(class_name)
    // }

    pub fn is_builtin(&self, ty_name: &str) -> bool {
        self.builtin_types.contains(ty_name)
    }

    pub fn is_singleton(&self, class_name: &str) -> bool {
        self.singletons.contains(class_name)
    }

    pub fn inheritance_tree(&self) -> &InheritanceTree {
        &self.inheritance_tree
    }

    pub fn find_rust_type(&'a self, ty: &str) -> Option<&'a RustTy> {
        self.cached_rust_types.get(ty)
    }

    pub fn insert_rust_type(&mut self, ty: &str, resolved: RustTy) {
        let prev = self.cached_rust_types.insert(ty.to_string(), resolved);
        assert!(prev.is_none(), "no overwrites of RustTy");
    }
}

#[derive(Default)]
pub(crate) struct InheritanceTree {
    derived_to_base: HashMap<String, String>,
}

impl InheritanceTree {
    pub fn insert(&mut self, derived: String, base: String) {
        let existing = self.derived_to_base.insert(derived, base);
        assert!(existing.is_none(), "Duplicate inheritance insert");
    }

    pub fn map_all_bases<T>(&self, derived: &str, apply: impl Fn(&str) -> T) -> Vec<T> {
        let mut maybe_base = derived;
        let mut result = vec![];
        loop {
            if let Some(base) = self.derived_to_base.get(maybe_base).map(String::as_str) {
                result.push(apply(base));
                maybe_base = base;
            } else {
                break;
            }
        }
        result
    }
}
