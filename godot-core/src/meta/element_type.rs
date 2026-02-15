/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt;

use crate::builtin::VariantType;
use crate::classes::Script;
use crate::meta::traits::{GodotType, element_variant_type};
use crate::meta::{ArrayElement, ClassId};
use crate::obj::{Gd, InstanceId};

/// Dynamic type information of Godot arrays and dictionaries.
///
/// Used in the following APIs:
/// - [`AnyArray::element_type()`][crate::builtin::AnyArray::element_type]
/// - [`Dictionary::key_element_type()`][crate::builtin::Dictionary::key_element_type]
/// - [`Dictionary::value_element_type()`][crate::builtin::Dictionary::value_element_type]
///
/// While Rust's type parameters provide compile-time type information, this method supplies additional RTTI (runtime type information).
/// For example, `Array<Gd<RefCounted>>` may store classes or scripts derived from `RefCounted`.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ElementType {
    /// Untyped array/dictionary that can contain any `Variant`.
    Untyped,

    /// Typed array with built-in type (e.g., `Array<i64>`, `Array<GString>`).
    Builtin(VariantType),

    /// Typed array with class (e.g., `Array<Gd<Node3D>>`, `Array<Gd<Resource>>`).
    Class(ClassId),

    /// Typed array with a script-based class (e.g. GDScript class `Enemy`).
    ///
    /// Arrays of this element type cannot be created directly in Rust code. They come into play when you have a GDScript with
    /// `class_name MyClass`, and then create a typed `Array[MyClass]` in GDScript. In Rust, these arrays can be represented with
    /// their _native base class_ (the one mentioned in `extends` in GDScript), e.g. `Array<Gd<RefCounted>>`.
    ScriptClass(ElementScript),
}

impl ElementType {
    /// Build element type info for a compile-time element `T`.
    pub fn of<T: ArrayElement>() -> Self {
        let variant_type = element_variant_type::<T>();
        if variant_type == VariantType::NIL {
            ElementType::Untyped
        } else if variant_type == VariantType::OBJECT {
            ElementType::Class(T::Via::class_id())
        } else {
            ElementType::Builtin(variant_type)
        }
    }

    /// True if this denotes a typed array/dictionary element.
    ///
    /// `Variant` is considered untyped, every other type is typed.
    pub fn is_typed(&self) -> bool {
        !matches!(self, ElementType::Untyped)
    }

    /// The `VariantType` corresponding to this element type.
    pub fn variant_type(&self) -> VariantType {
        match self {
            ElementType::Untyped => VariantType::NIL,
            ElementType::Builtin(variant_type) => *variant_type,
            ElementType::Class(_) | ElementType::ScriptClass(_) => VariantType::OBJECT,
        }
    }

    /// The class ID, if this type is of type [`Class`][ElementType::Class] or [`ScriptClass`][ElementType::ScriptClass].
    pub fn class_id(&self) -> Option<ClassId> {
        match self {
            ElementType::Class(class_name) => Some(*class_name),
            ElementType::ScriptClass(script) => script.base_class_id(),
            _ => None,
        }
    }

    /// Returns the class name sys pointer for FFI calls like `array_set_typed` / `dictionary_set_typed`.
    ///
    /// If `self` has a class ID, returns its `string_sys()`. Otherwise, returns `fallback.string_sys()`.
    /// The caller must keep `fallback` (typically `StringName::default()`) alive while the returned pointer is in use.
    pub(crate) fn class_name_sys_or(
        &self,
        fallback: &crate::builtin::StringName,
    ) -> crate::sys::GDExtensionConstStringNamePtr {
        if let Some(class_id) = self.class_id() {
            class_id.string_sys()
        } else {
            fallback.string_sys()
        }
    }

    /// Checks if `self` (runtime type) is compatible with `expected` (compile-time type).
    ///
    /// Returns `true` if:
    /// - The types match exactly, OR
    /// - `self` is a `ScriptClass` and `expected` is its native base `Class`
    ///
    /// This allows an `Array[Enemy]` from GDScript (where `Enemy extends RefCounted`) to be used as `Array<Gd<RefCounted>>` in Rust.
    /// TODO(v0.6): this breaks covariance -- consider using generic AnyArray<Gd<RefCounted>> instead.
    pub(crate) fn is_compatible_with(&self, expected: &ElementType) -> bool {
        // Exact match.
        if self == expected {
            return true;
        }

        // Script class (runtime) matches its native base class (compile-time).
        matches!(
            (self, expected),
            (ElementType::ScriptClass(_), ElementType::Class(expected_class))
                if self.class_id().is_some_and(|id| id == *expected_class)
        )
    }

    /// Transfer cached element type from source to destination, preserving type info.
    ///
    /// Used by clone-like operations like `duplicate()`, `slice()`, etc. where we want to preserve cached type information to avoid
    /// redundant FFI calls. Only transfers if the source has computed info and destination doesn't.
    pub(crate) fn transfer_cache(
        source_cache: &std::cell::OnceCell<ElementType>,
        dest_cache: &std::cell::OnceCell<ElementType>,
    ) {
        if let Some(source_value) = source_cache.get() {
            // Ignore result. If dest is already set, that's fine.
            let _ = dest_cache.set(*source_value);
        }
    }

    /// Get element type from cache or compute it via FFI calls.
    ///
    /// Returns cached value if available, otherwise computes via FFI and caches the result.
    ///
    /// In Debug mode, validates cached `Untyped` values to ensure another extension hasn't made an array/dictionary typed via C functions
    /// `set_array_type`/`set_dictionary_type` after caching.
    ///
    /// Takes closures for the three FFI operations needed to determine element type:
    /// - `get_builtin_type`: Get the variant type (sys variant type as i64)
    /// - `get_class_name`: Get the class name as StringName
    /// - `get_script_variant`: Get the script variant directly
    ///
    /// Returns the computed `ElementType` and updates the cache.
    pub(crate) fn get_or_compute_cached(
        cache: &std::cell::OnceCell<ElementType>,
        get_builtin_type: impl Fn() -> i64,
        get_class_name: impl Fn() -> crate::builtin::StringName,
        get_script_variant: impl Fn() -> crate::builtin::Variant,
    ) -> ElementType {
        let cached = *cache.get_or_init(|| {
            let sys_variant_type = get_builtin_type();
            let variant_type =
                VariantType::from_sys(sys_variant_type as crate::sys::GDExtensionVariantType);

            if variant_type == VariantType::NIL {
                ElementType::Untyped
            } else if variant_type == VariantType::OBJECT {
                let class_name_stringname = get_class_name();
                let class_name = ClassId::new_dynamic(class_name_stringname.to_string());

                // If there's a script associated, the class is interpreted as the native base class of the script.
                let script_variant = get_script_variant();
                match Self::script_from_variant(&script_variant) {
                    Some(script) => ElementType::ScriptClass(ElementScript::new(script)),
                    _ => ElementType::Class(class_name),
                }
            } else {
                ElementType::Builtin(variant_type)
            }
        });

        // Consistency validation for cached Untyped values.
        #[cfg(safeguards_strict)]
        if matches!(cached, ElementType::Untyped) {
            let sys_variant_type = get_builtin_type();
            let variant_type =
                VariantType::from_sys(sys_variant_type as crate::sys::GDExtensionVariantType);

            assert_eq!(
                variant_type,
                VariantType::NIL,
                "Array/Dictionary element type validation failed: cached as Untyped but FFI reports {variant_type:?}. \
                This indicates that another extension modified the type after godot-rust cached it.",
            );
        }

        cached
    }

    /// Convert a script variant to a `Gd<Script>`, or `None` if nil.
    fn script_from_variant(script_variant: &crate::builtin::Variant) -> Option<Gd<Script>> {
        use crate::meta::FromGodot;

        if script_variant.get_type() == VariantType::NIL {
            None
        } else {
            Gd::<Script>::try_from_variant(script_variant).ok()
        }
    }
}

impl fmt::Debug for ElementType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ElementType::Untyped => write!(f, "Untyped"),
            ElementType::Builtin(variant_type) => {
                write!(f, "Builtin({:?})", variant_type)
            }
            ElementType::Class(class_name) => {
                write!(f, "Class({})", class_name)
            }
            ElementType::ScriptClass(script) => match script.script() {
                // Script::get_global_name() is only available in Godot 4.3+.
                #[cfg(before_api = "4.3")]
                Some(s) => {
                    write!(f, "ScriptClass(? extends {})", s.get_instance_base_type())
                }

                #[cfg(since_api = "4.3")]
                Some(s) => {
                    let script_name = s.get_global_name().to_string();
                    if script_name.is_empty() {
                        write!(f, "ScriptClass(? extends {})", s.get_instance_base_type())
                    } else {
                        write!(f, "ScriptClass({})", script_name)
                    }
                }

                None => write!(f, "ScriptClass(<Freed Object>)"),
            },
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// ElementScript

/// Compact representation inside [`ElementType::ScriptClass`].
///
/// Encapsulates a `Gd<Script>`, obtained via [`script()`][Self::script].
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct ElementScript {
    /// Weak pointer to `Gd<Script>`.
    script_instance_id: InstanceId,
}

impl ElementScript {
    /// Create a new `ElementScriptType` from a script.
    pub fn new(script: Gd<Script>) -> Self {
        Self {
            script_instance_id: script.instance_id(),
        }
    }

    /// Returns the script object, if still alive.
    ///
    /// The script is a `Resource` and won't be kept alive by this type-info struct. If the resource has been deallocated,
    /// this method returns `None`.
    pub fn script(&self) -> Option<Gd<Script>> {
        // Note: might also fail in the future if acquired on another thread.
        Gd::try_from_instance_id(self.script_instance_id).ok()
    }

    /// Returns the native base class of the script.
    ///
    /// Typically, this corresponds to the class mentioned in `extends` in GDScript.
    pub fn base_class_id(&self) -> Option<ClassId> {
        self.script().map(|s| {
            let base_type = s.get_instance_base_type();
            ClassId::new_dynamic(base_type.to_string())
        })
    }
}
