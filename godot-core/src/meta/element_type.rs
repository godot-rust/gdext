/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt;

use crate::builtin::{StringName, VariantType};
use crate::classes::Script;
use crate::meta::traits::{element_variant_type, GodotType};
use crate::meta::{ArrayElement, ClassName};
use crate::obj::{Gd, InstanceId};

/// Dynamic type information of Godot arrays and dictionaries.
///
/// Used by [`Array::element_type()`][Array::element_type], [`Dictionary::key_type()`][Dictionary::key_type] and
/// [`Dictionary::value_type()`][Dictionary::value_type].
///
/// While Rust's type parameters provide compile-time type information, this method can give additional RTTI (runtime type information).
/// For example, `Array<Gd<RefCounted>>` may store classes or scripts derived from `RefCounted`.
///
/// **Thread Safety**: This type is not `Send + Sync` due to the `ScriptClass` variant containing `Gd<Script>`.
/// For error handling and other contexts requiring thread-safe type info, use [`ThreadSafeElementType`] instead.
#[derive(Clone, PartialEq, Eq)]
pub enum ElementType {
    /// Untyped array/dictionary that can contain any `Variant`.
    Untyped,

    /// Typed array with built-in type (e.g., `Array<i64>`, `Array<GString>`).
    Builtin(VariantType),

    /// Typed array with class (e.g., `Array<Gd<Node3D>>`, `Array<Gd<Resource>>`).
    Class(ClassName),

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
            ElementType::Class(T::Via::class_name())
        } else {
            ElementType::Builtin(variant_type)
        }
    }

    /// Construct from runtime information (variant type and optional class name).
    pub(crate) fn from_runtime(variant_type: VariantType, class_name: Option<StringName>) -> Self {
        if variant_type == VariantType::NIL {
            ElementType::Untyped
        } else if variant_type == VariantType::OBJECT {
            let class_name = class_name
                .map(|name| ClassName::new_dynamic(name.to_string()))
                .unwrap_or_else(ClassName::none);

            ElementType::Class(class_name)
        } else {
            ElementType::Builtin(variant_type)
        }
    }

    /// True if this denotes a typed array (non-NIL variant type).
    pub fn is_typed(&self) -> bool {
        !matches!(self, ElementType::Untyped)
    }

    /// The VariantType corresponding to this element type.
    pub fn variant_type(&self) -> VariantType {
        match self {
            ElementType::Untyped => VariantType::NIL,
            ElementType::Builtin(variant_type) => *variant_type,
            ElementType::Class(_) | ElementType::ScriptClass(_) => VariantType::OBJECT,
        }
    }

    /// The class name if this is a class-typed array.
    pub fn class_name(&self) -> Option<ClassName> {
        match self {
            ElementType::Class(class_name) => Some(*class_name),
            ElementType::ScriptClass(script) => {
                // For script classes, we return the native base class name
                script.script().map(|s| {
                    let base_type = s.get_instance_base_type();
                    ClassName::new_dynamic(base_type.to_string())
                })
            }
            _ => None,
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
            ElementType::ScriptClass(script) => {
                if let Some(s) = script.script() {
                    write!(f, "ScriptClass({:?})", s)
                } else {
                    write!(f, "ScriptClass(<invalid>)")
                }
            }
        }
    }
}

/// Backward-compat: keep ArrayTypeInfo name as an alias.
pub(crate) type ArrayTypeInfo = ElementType;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// ElementScript

/// Compact representation inside [`ElementType::ScriptClass`].
///
/// Encapsulates a `Gd<Script>`, obtained via [`script()`][Self::script].
#[derive(Clone, PartialEq, Eq)]
pub struct ElementScript {
    /// Weak pointer to Gd<Script>.
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
}
