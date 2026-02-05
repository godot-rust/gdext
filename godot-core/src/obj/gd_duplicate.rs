/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Type-safe duplicate methods for Node and Resource.

use crate::classes::node::DuplicateFlags;
#[cfg(since_api = "4.5")]
use crate::classes::resource::DeepDuplicateMode;
use crate::classes::{Node, Resource};
use crate::obj::{Gd, Inherits};

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Node duplication

impl<T> Gd<T>
where
    T: Inherits<Node>,
{
    /// ⚠️ Returns a new node with all of its properties, signals, groups, and children copied from the original.
    ///
    /// See [`duplicate_node_ex()`][Self::duplicate_node_ex] for details and panics.
    ///
    /// # Example
    /// ```no_run
    /// use godot::prelude::*;
    ///
    /// let mut node = Node2D::new_alloc();
    /// node.set_position(Vector2::new(1.0, 2.0));
    ///
    /// let copy = node.duplicate_node(); // type Gd<Node2D>
    /// assert_eq!(copy.get_position(), Vector2::new(1.0, 2.0));
    ///
    /// node.free();
    /// copy.free();
    /// ```
    pub fn duplicate_node(&self) -> Gd<T> {
        self.duplicate_node_ex().done()
    }

    /// Duplicates this node with a fluent builder API for fine-grained control.
    ///
    /// By default, all flags are enabled, just like [`duplicate_node()`][Self::duplicate_node]:
    /// - Properties, signals, groups, and children are copied.
    /// - Internal nodes are not duplicated.
    ///
    /// You can change this behavior with [`flags()`][ExDuplicateNode::flags]. For nodes with attached scripts: if the script's `_init()` has
    /// required parameters, the duplicated node will **not** have a script.
    ///
    /// This function can be used polymorphically: duplicating `Gd<Node>` pointing to dynamic type `Node2D` duplicates the concrete `Node2D`.
    ///
    /// # Panics
    /// Panics if duplication fails. Likely causes:
    /// - The **dynamic** (runtime) type of the node is not default-constructible. For example, if a `Gd<Node>` actually points to an instance
    ///   of `MyClass` (inheriting `Node`), and `MyClass` has no `init` or uses `#[class(no_init)]`, then duplication will fail.
    /// - Called from a thread other than the main thread (Godot's scene tree is single-threaded).
    /// - You use [`DuplicateFlags::USE_INSTANTIATION`] and the scene file cannot be loaded.
    /// - Any child node's duplication fails.
    ///
    /// To avoid panics, use [`ExDuplicateNode::done_or_null()`].
    ///
    /// # Example
    /// ```no_run
    /// use godot::prelude::*;
    /// use godot::classes::node::DuplicateFlags;
    ///
    /// let node: Gd<Node> = Node::new_alloc();
    /// // Configure node...
    /// let copy = node.duplicate_node_ex()
    ///     .flags(DuplicateFlags::SIGNALS | DuplicateFlags::GROUPS)
    ///     .done();
    /// ```
    pub fn duplicate_node_ex(&self) -> ExDuplicateNode<'_, T> {
        ExDuplicateNode::new(self)
    }
}

/// Builder for duplicating a node with fine-grained flag control.
///
/// Created by [`Gd::duplicate_node_ex()`].
/// See [`duplicate_node_ex()`](Gd::duplicate_node_ex) for complete documentation including
/// default behavior, panic conditions, and script handling.
#[must_use]
pub struct ExDuplicateNode<'a, T>
where
    T: Inherits<Node>,
{
    node: &'a Gd<T>,
    flags: DuplicateFlags,
}

impl<'a, T> ExDuplicateNode<'a, T>
where
    T: Inherits<Node>,
{
    fn new(node: &'a Gd<T>) -> Self {
        Self {
            node,
            // DEFAULT constant is only available from Godot 4.5. Any bits unrecognized by earlier versions should be ignored by Godot.
            flags: crate::obj::EngineBitfield::from_ord(15),
        }
    }

    /// **Replaces** flags (use `|` to combine them).
    pub fn flags(mut self, flags: DuplicateFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Complete the duplication and return the duplicated node.
    ///
    /// # Panics
    /// On errors, see [`Gd::duplicate_node_ex()`]. To check for results, use [`done_or_null()`][Self::done_or_null].
    pub fn done(self) -> Gd<T> {
        self.try_duplicate()
            .and_then(|dup| dup.try_cast::<T>().ok())
            .unwrap_or_else(|| {
                panic!(
                    "Failed to duplicate class {t}; is it default-constructible?",
                    t = self.node.dynamic_class_string()
                )
            })
    }

    /// Complete the duplication and return the duplicated node, or `None` if it fails.
    ///
    /// See [`Gd::duplicate_node_ex()`] for details.
    pub fn done_or_null(self) -> Option<Gd<T>> {
        self.try_duplicate()?.try_cast::<T>().ok()
    }

    fn try_duplicate(&self) -> Option<Gd<Node>> {
        #[expect(deprecated)]
        self.node.upcast_ref::<Node>().duplicate_full(self.flags)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Resource duplication

impl<T> Gd<T>
where
    T: Inherits<Resource>,
{
    /// ⚠️ Returns a shallow duplicate of this resource.
    ///
    /// See [`duplicate_resource_ex()`][Self::duplicate_resource_ex] for details, panics, and version-specific behavior.
    ///
    /// # Example
    /// ```no_run
    /// use godot::prelude::*;
    ///
    /// let resource = Resource::new_gd();
    /// let copy = resource.duplicate_resource(); // Gd<Resource>.
    /// assert_ne!(copy, resource); // Different Gd pointer.
    /// ```
    pub fn duplicate_resource(&self) -> Gd<T> {
        self.duplicate_resource_ex().done()
    }

    /// Duplicates this resource with a fluent builder API for fine-grained control.
    ///
    /// By default, performs a shallow copy, same as [`duplicate_resource()`][Self::duplicate_resource].  \
    /// Use [`deep_internal()`][ExDuplicateResource::deep_internal] or [`deep()`][ExDuplicateResource::deep] to control
    /// subresource duplication.
    ///
    /// Works polymorphically: duplicating `Gd<Resource>` pointing to a dynamic type duplicates the concrete type.
    ///
    /// # Panics
    /// If the dynamic type is not default-constructible (e.g. `#[class(no_init)]`).
    /// Use [`ExDuplicateResource::done_or_null()`] to handle errors.
    ///
    /// # Behavior table
    /// The behavior has changed in [PR #100673](https://github.com/godotengine/godot/pull/100673) for Godot 4.5, and with both `duplicate()`
    /// and `duplicate_deep()`, there is now partial semantic overlap in Godot. The following table summarizes the behavior and elaborates
    /// how it maps to godot-rust.
    ///
    /// See also [Godot docs](https://docs.godotengine.org/en/stable/classes/class_resource.html#class-resource-method-duplicate)
    /// for `Resource.duplicate()` and `Resource.duplicate_deep()`.
    ///
    /// | godot-rust | Godot | 4.2–4.4 | 4.5+ |
    /// |------------|-------|---------|------|
    /// | `duplicate_resource()`<br>`duplicate_resource_ex()` | `duplicate(false)` | A/D[^ad] shallow-copied, subresources shared | A/D only referenced |
    /// | `_ex().deep_internal()` | `duplicate(true)` | A/D shallow-copied, subresources in A/D<br>**ignored** (bug) | A/D deep-copied, **internal** subresources duplicated[^internal] |
    /// | `_ex().deep(NONE)` | `duplicate_deep(NONE)` | | A/D deep-copied, subresources **shared** |
    /// | `_ex().deep(INTERNAL)` | `duplicate_deep(INTERNAL)` | | A/D deep-copied, **internal** subresources **duplicated** |
    /// | `_ex().deep(ALL)` | `duplicate_deep(ALL)` | | A/D deep-copied, **all** subresources **duplicated** |
    ///
    /// [^ad]: "A/D" stands for "`Array`/`Dictionary`".
    /// [^internal]: `_ex().deep_internal()` is equivalent to `_ex().deep(INTERNAL)`. This method only exists for <4.5 compatibility.
    pub fn duplicate_resource_ex(&self) -> ExDuplicateResource<'_, T> {
        ExDuplicateResource::new(self)
    }
}

/// Builder for duplicating a resource with deep duplication control.
///
/// Created by [`Gd::duplicate_resource_ex()`]. See that method for details and version-specific behavior.
///
/// Configuration methods:
/// - [`deep_internal()`][Self::deep_internal]: Duplicates internal subresources (all Godot versions).
/// - [`deep(subresources)`][Self::deep]: Fine-grained control via [`DeepDuplicateMode`] (**Godot 4.5+**).
///
/// Terminal methods:
/// - [`done()`][Self::done]: Finalize duplication (panics on failure).
/// - [`done_or_null()`][Self::done_or_null]: Finalize duplication (returns `Result`).
#[must_use]
pub struct ExDuplicateResource<'a, T>
where
    T: Inherits<Resource>,
{
    resource: &'a Gd<T>,
    godot_api: GodotDuplicateApi,
}

enum GodotDuplicateApi {
    Duplicate {
        deep: bool,
    },
    #[cfg(since_api = "4.5")]
    DuplicateDeep {
        mode: crate::classes::resource::DeepDuplicateMode,
    },
}

impl<'a, T> ExDuplicateResource<'a, T>
where
    T: Inherits<Resource>,
{
    fn new(resource: &'a Gd<T>) -> Self {
        Self {
            resource,
            godot_api: GodotDuplicateApi::Duplicate { deep: false },
        }
    }

    /// Deep duplication of internal subresources (Godot 4.3+).
    ///
    /// Duplicates subresources that have no external path (embedded/internal resources).
    ///
    /// # Compatibility
    /// In Godot 4.2-4.4, subresources inside `Array`/`Dictionary` properties are **not** duplicated (known Godot bug).
    /// This was fixed in Godot 4.5, from which version onward this is equivalent to [`deep(DeepDuplicateMode::INTERNAL)`][Self::deep].
    // Needs to exist as a separate method for <4.5 compatibility.
    pub fn deep_internal(mut self) -> Self {
        self.godot_api = GodotDuplicateApi::Duplicate { deep: true };
        self
    }

    /// Deep duplication with control over subresources (**Godot 4.5+**).
    ///
    /// The `subresources` parameter controls which `Resource` objects are duplicated:
    /// - [`NONE`][DeepDuplicateMode::NONE]: No subresources duplicated (but `Array`/`Dictionary` containers are deep-copied).
    /// - [`INTERNAL`][DeepDuplicateMode::INTERNAL]: Duplicates internal subresources only (no external path). Same as [`deep_internal()`][Self::deep_internal].
    /// - [`ALL`][DeepDuplicateMode::ALL]: Duplicates all subresources.
    ///
    /// Note: Unlike [`done()`][Self::done] without `deep()`, this method **always** deep-copies `Array`/`Dictionary` containers.
    /// The `subresources` mode only controls whether `Resource` objects inside them are duplicated or shared.
    ///
    /// # Compatibility
    /// Requires Godot 4.5, as it uses Godot's new `Resource::duplicate_deep()` API.
    #[cfg(since_api = "4.5")]
    pub fn deep(mut self, subresources: DeepDuplicateMode) -> Self {
        self.godot_api = GodotDuplicateApi::DuplicateDeep { mode: subresources };
        self
    }

    /// Complete the duplication and return the duplicated resource.
    ///
    /// # Panics
    /// On errors, see [`Gd::duplicate_resource_ex()`]. To check for results, use [`done_or_null()`][Self::done_or_null].
    pub fn done(self) -> Gd<T> {
        self.try_duplicate()
            .and_then(|dup| dup.try_cast::<T>().ok())
            .unwrap_or_else(|| {
                panic!(
                    "Failed to duplicate class {t}; is it default-constructible?",
                    t = self.resource.dynamic_class_string()
                )
            })
    }

    /// Complete the duplication and return the duplicated resource, or `None` if it fails.
    ///
    /// See [`Gd::duplicate_resource_ex()`] for details.
    pub fn done_or_null(self) -> Option<Gd<T>> {
        self.try_duplicate()?.try_cast::<T>().ok()
    }

    #[expect(deprecated)]
    fn try_duplicate(&self) -> Option<Gd<Resource>> {
        let resource_ref = self.resource.upcast_ref::<Resource>();

        match self.godot_api {
            // Godot 4.5 renamed parameter, so our default-param method is different.
            #[cfg(since_api = "4.5")]
            GodotDuplicateApi::Duplicate { deep } => resource_ref.duplicate_ex().deep(deep).done(),

            #[cfg(before_api = "4.5")]
            GodotDuplicateApi::Duplicate { deep } => {
                resource_ref.duplicate_ex().subresources(deep).done()
            }

            #[cfg(since_api = "4.5")]
            GodotDuplicateApi::DuplicateDeep { mode } => resource_ref
                .duplicate_deep_ex()
                .deep_subresources_mode(mode)
                .done(),
        }
    }
}
