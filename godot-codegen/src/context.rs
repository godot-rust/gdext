/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::{HashMap, HashSet};

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, ToTokens};

use crate::generator::method_tables::MethodTableKey;
use crate::generator::notifications;
use crate::models::domain::{ArgPassing, GodotTy, RustTy, TyName};
use crate::models::json::{
    JsonBuiltinClass, JsonBuiltinMethod, JsonClass, JsonClassConstant, JsonClassMethod,
};
use crate::util::option_as_slice;
use crate::{special_cases, util, JsonExtensionApi};

#[derive(Default)]
pub struct Context<'a> {
    builtin_types: HashSet<&'a str>,
    native_structures_types: HashSet<&'a str>,
    singletons: HashSet<&'a str>,
    inheritance_tree: InheritanceTree,
    /// Which interface traits are generated (`false` for "Godot-abstract"/final classes).
    classes_final: HashMap<TyName, bool>,
    cached_rust_types: HashMap<GodotTy, RustTy>,
    notifications_by_class: HashMap<TyName, Vec<(Ident, i32)>>,
    classes_with_signals: HashSet<TyName>,
    notification_enum_names_by_class: HashMap<TyName, NotificationEnum>,
    method_table_indices: HashMap<MethodTableKey, usize>,
    method_table_next_index: HashMap<String, usize>,
}

impl<'a> Context<'a> {
    pub fn build_from_api(api: &'a JsonExtensionApi) -> Self {
        let mut ctx = Self::default();

        for class in api.singletons.iter() {
            ctx.singletons.insert(class.name.as_str());
        }

        ctx.builtin_types.insert("Variant"); // not part of builtin_classes
        for builtin in api.builtin_classes.iter() {
            let ty_name = builtin.name.as_str();
            ctx.builtin_types.insert(ty_name);

            Self::populate_builtin_class_table_indices(
                builtin,
                option_as_slice(&builtin.methods),
                &mut ctx,
            );
        }

        for structure in api.native_structures.iter() {
            let ty_name = structure.name.as_str();
            ctx.native_structures_types.insert(ty_name);
        }

        let mut engine_classes = HashMap::new();
        for class in api.classes.iter() {
            let class_name = TyName::from_godot(&class.name);

            if special_cases::is_class_deleted(&class_name) {
                continue;
            }

            // Populate class lookup by name.
            engine_classes.insert(class_name.clone(), class);

            if !option_as_slice(&class.signals).is_empty() {
                ctx.classes_with_signals.insert(class_name.clone());
            }

            ctx.classes_final
                .insert(class_name.clone(), ctx.is_class_final(&class_name));

            // Populate derived-to-base relations
            if let Some(base) = class.inherits.as_ref() {
                let base_name = TyName::from_godot(base);
                // println!(
                //     "* Add engine class {} <- inherits {}",
                //     class_name.description(),
                //     base_name.description()
                // );
                ctx.inheritance_tree.insert(class_name.clone(), base_name);
            } else {
                // println!("* Add engine class {}", class_name.description());
            }

            // Populate notification constants (first, only for classes that declare them themselves).
            Self::populate_notification_constants(
                &class_name,
                option_as_slice(&class.constants),
                &mut ctx,
            );

            Self::populate_class_table_indices(
                class,
                &class_name,
                option_as_slice(&class.methods),
                &mut ctx,
            );
        }

        // Populate remaining notification enum names, by copying the one to nearest base class that has at least 1 notification.
        // At this point all classes with notifications are registered.
        // (Used to avoid re-generating the same notification enum for multiple base classes).
        for class_name in engine_classes.keys() {
            if ctx
                .notification_enum_names_by_class
                .contains_key(class_name)
            {
                continue;
            }

            let all_bases = ctx.inheritance_tree.collect_all_bases(class_name);

            let mut nearest = None;
            for (i, elem) in all_bases.iter().enumerate() {
                if let Some(nearest_enum_name) = ctx.notification_enum_names_by_class.get(elem) {
                    nearest = Some((i, nearest_enum_name.clone()));
                    break;
                }
            }
            let (nearest_index, nearest_enum_name) = nearest.unwrap_or_else(|| {
                panic!(
                    "class {}: at least one base must have notifications; possibly, its direct base has been removed from minimal codegen",
                    class_name.godot_ty
                )
            });

            // For all bases inheriting most-derived base that has notification constants, reuse the type name.
            for i in (0..nearest_index).rev() {
                let base_name = &all_bases[i];
                let enum_name = NotificationEnum::for_other_class(nearest_enum_name.clone());

                ctx.notification_enum_names_by_class
                    .insert(base_name.clone(), enum_name);
            }

            // Also for this class, reuse the type name.
            let enum_name = NotificationEnum::for_other_class(nearest_enum_name);

            ctx.notification_enum_names_by_class
                .insert(class_name.clone(), enum_name);
        }

        ctx
    }

    fn populate_notification_constants(
        class_name: &TyName,
        constants: &[JsonClassConstant],
        ctx: &mut Context,
    ) {
        let mut has_notifications = false;
        for constant in constants.iter() {
            if let Some(rust_constant) = notifications::try_to_notification(constant) {
                // First time
                if !has_notifications {
                    ctx.notifications_by_class
                        .insert(class_name.clone(), Vec::new());

                    ctx.notification_enum_names_by_class.insert(
                        class_name.clone(),
                        NotificationEnum::for_own_class(class_name),
                    );

                    has_notifications = true;
                }

                ctx.notifications_by_class
                    .get_mut(class_name)
                    .expect("just inserted constants; must be present")
                    .push((rust_constant, constant.to_enum_ord()));
            }
        }
    }

    fn populate_class_table_indices(
        class: &JsonClass,
        class_name: &TyName,
        methods: &[JsonClassMethod],
        ctx: &mut Context,
    ) {
        // Note: already checked for class excluded/deleted.

        for method in methods.iter() {
            if special_cases::is_class_method_deleted(class_name, method, ctx) || method.is_virtual
            {
                continue;
            }

            let key = MethodTableKey::ClassMethod {
                api_level: util::get_api_level(class),
                class_ty: class_name.clone(),
                method_name: method.name.clone(),
            };

            ctx.register_table_index(key);
        }
    }

    fn populate_builtin_class_table_indices(
        builtin: &JsonBuiltinClass,
        methods: &[JsonBuiltinMethod],
        ctx: &mut Context,
    ) {
        let builtin_ty = TyName::from_godot(builtin.name.as_str());
        if special_cases::is_builtin_type_deleted(&builtin_ty) {
            return;
        }

        for method in methods.iter() {
            if special_cases::is_builtin_method_deleted(&builtin_ty, method) {
                continue;
            }

            let key = MethodTableKey::BuiltinMethod {
                builtin_ty: builtin_ty.clone(),
                method_name: method.name.clone(),
            };

            ctx.register_table_index(key);
        }
    }

    // Private, because initialized in constructor. Ensures deterministic assignment.
    fn register_table_index(&mut self, key: MethodTableKey) -> usize {
        let key_category = key.category();

        let next_index = self
            .method_table_next_index
            .entry(key_category)
            .or_insert(0);

        let prev = self.method_table_indices.insert(key, *next_index);
        assert!(prev.is_none(), "table index already registered");

        *next_index += 1;
        *next_index
    }

    pub fn get_table_index(&self, key: &MethodTableKey) -> usize {
        *self
            .method_table_indices
            .get(key)
            .unwrap_or_else(|| panic!("did not register table index for key {key:?}"))
    }

    /// Yields cached sys pointer types – various pointer types declared in `gdextension_interface`
    /// and used as parameters in exposed Godot APIs.
    pub fn cached_sys_pointer_types(&self) -> impl Iterator<Item = &RustTy> {
        self.cached_rust_types
            .values()
            .filter(|rust_ty| rust_ty.is_sys_pointer())
    }

    /// Whether an interface trait is generated for a class.
    ///
    /// False if the class is "Godot-abstract"/final, thus there are no virtual functions to inherit.
    fn is_class_final(&self, class_name: &TyName) -> bool {
        debug_assert!(
            !self.singletons.is_empty(),
            "initialize singletons before final-check"
        );

        self.singletons.contains(class_name.godot_ty.as_str())
            || special_cases::is_class_abstract(class_name)
    }

    // ------------------------------------------------------------------------------------------------------------------------------------------

    /// Checks if this is a builtin type (not `Object`).
    ///
    /// Note that builtins != variant types.
    pub fn is_builtin(&self, ty_name: &str) -> bool {
        self.builtin_types.contains(ty_name)
    }

    pub fn get_builtin_arg_passing(&self, godot_ty: &GodotTy) -> ArgPassing {
        // Already handled separately.
        debug_assert!(!godot_ty.ty.starts_with("Packed"));

        // IMPORTANT: Keep this in sync with impl_ffi_variant!() macros taking `ref` or not.

        // Arrays are also handled separately, and use ByRef.
        match godot_ty.ty.as_str() {
            // Note: Signal is currently not used in any parameter, but this may change.
            "Variant" | "Array" | "Dictionary" | "Callable" | "Signal" => ArgPassing::ByRef,
            "String" | "StringName" | "NodePath" => ArgPassing::ImplAsArg,
            _ => ArgPassing::ByValue,
        }
    }

    pub fn is_native_structure(&self, ty_name: &str) -> bool {
        self.native_structures_types.contains(ty_name)
    }

    pub fn is_singleton(&self, class_name: &TyName) -> bool {
        self.singletons.contains(class_name.godot_ty.as_str())
    }

    pub fn is_final(&self, class_name: &TyName) -> bool {
        *self.classes_final.get(class_name).unwrap_or_else(|| {
            panic!(
                "queried final status for class {}, but it is not registered",
                class_name.godot_ty
            )
        })
    }

    pub fn inheritance_tree(&self) -> &InheritanceTree {
        &self.inheritance_tree
    }

    pub fn find_rust_type(&'a self, ty: &GodotTy) -> Option<&'a RustTy> {
        self.cached_rust_types.get(ty)
    }

    /// Walks up in the hierarchy, and returns the first (nearest) base class which declares at least 1 signal.
    ///
    /// Always returns a result, as `Object` (the root) itself declares signals.
    pub fn find_nearest_base_with_signals(&self, class_name: &TyName) -> TyName {
        let tree = self.inheritance_tree();

        let mut class = class_name.clone();
        while let Some(base) = tree.direct_base(&class) {
            if self.classes_with_signals.contains(&base) {
                return base;
            } else {
                class = base;
            }
        }

        panic!("Object (root) should always have signals")
    }

    pub fn notification_constants(&'a self, class_name: &TyName) -> Option<&'a Vec<(Ident, i32)>> {
        self.notifications_by_class.get(class_name)
    }

    pub fn notification_enum_name(&self, class_name: &TyName) -> NotificationEnum {
        self.notification_enum_names_by_class
            .get(class_name)
            .unwrap_or_else(|| panic!("class {} has no notification enum name", class_name.rust_ty))
            .clone()
    }

    pub fn insert_rust_type(&mut self, godot_ty: GodotTy, resolved: RustTy) {
        let prev = self.cached_rust_types.insert(godot_ty, resolved);
        assert!(prev.is_none(), "no overwrites of RustTy");
    }
}
// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(Clone)]
pub struct NotificationEnum {
    /// Name of the enum.
    pub name: Ident,

    /// Whether this is declared by the current class (from context), rather than inherited.
    pub declared_by_own_class: bool,
}

impl NotificationEnum {
    fn for_own_class(class_name: &TyName) -> Self {
        Self {
            name: format_ident!("{}Notification", class_name.rust_ty),
            declared_by_own_class: true,
        }
    }

    fn for_other_class(other: NotificationEnum) -> Self {
        Self {
            name: other.name,
            declared_by_own_class: false,
        }
    }

    /// Returns the name of the enum if it is declared by the current class, or `None` if it is inherited.
    pub fn try_to_own_name(&self) -> Option<Ident> {
        if self.declared_by_own_class {
            Some(self.name.clone())
        } else {
            None
        }
    }
}

impl ToTokens for NotificationEnum {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.name.to_tokens(tokens)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Maintains class hierarchy. Uses Rust class names, not Godot ones.
#[derive(Default)]
pub struct InheritanceTree {
    derived_to_base: HashMap<TyName, TyName>,
}

impl InheritanceTree {
    pub fn insert(&mut self, derived_name: TyName, base_name: TyName) {
        let existing = self.derived_to_base.insert(derived_name, base_name);
        assert!(existing.is_none(), "Duplicate inheritance insert");
    }

    #[allow(unused)] // Currently 4.4 gated, for virtual method hashes.
    pub fn direct_base(&self, derived_name: &TyName) -> Option<TyName> {
        self.derived_to_base.get(derived_name).cloned()
    }

    /// Returns all base classes, without the class itself, in order from nearest to furthest (`Object`).
    pub fn collect_all_bases(&self, derived_name: &TyName) -> Vec<TyName> {
        let mut upgoer = derived_name;
        let mut result = vec![];

        while let Some(base) = self.derived_to_base.get(upgoer) {
            result.push(base.clone());
            upgoer = base;
        }
        result
    }

    /// Whether a class is a direct or indirect subclass of another (true for derived == base).
    pub fn inherits(&self, derived: &TyName, base_name: &str) -> bool {
        // Reflexive: T inherits T.
        if derived.godot_ty == base_name {
            return true;
        }

        let mut upgoer = derived;

        while let Some(next_base) = self.derived_to_base.get(upgoer) {
            if next_base.godot_ty == base_name {
                return true;
            }
            upgoer = next_base;
        }

        false
    }
}
