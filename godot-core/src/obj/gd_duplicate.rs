/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Type-safe duplicate methods for Node and Resource.

use crate::classes::node::DuplicateFlags;
use crate::classes::resource::DeepDuplicateMode;
use crate::classes::{Node, Resource};
use crate::obj::{EngineBitfield, Gd, Inherits};

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
    /// To avoid panics, use [`ExDuplicateNode::try_done()`].
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
            flags: DuplicateFlags::from_ord(15), // Start with all flags (same as duplicate_node())
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
    /// On errors, see [`Gd::duplicate_node_ex()`]. To check for results, use [`try_done()`][Self::try_done].
    pub fn done(self) -> Gd<T> {
        let type_name = T::class_id();
        self.try_done()
            .unwrap_or_else(|()| panic!("Failed to duplicate {}", type_name))
    }

    /// Complete the duplication and return the duplicated node, or an error if it fails.
    ///
    /// See [`Gd::duplicate_node_ex()`] for details.
    pub fn try_done(self) -> Result<Gd<T>, ()> {
        self.node
            .upcast_ref::<Node>()
            .duplicate_full(self.flags)
            .ok_or(())?
            .try_cast::<T>()
            .map_err(|_| ())
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Resource duplication

impl<T> Gd<T>
where
    T: Inherits<Resource>,
{
    /// ⚠️ Returns a shallow duplicate of this resource (subresources are shared).
    ///
    /// See [`duplicate_resource_ex()`][Self::duplicate_resource_ex] for details and panics.
    ///
    /// # Example
    /// ```no_run
    /// use godot::prelude::*;
    ///
    /// let resource = Resource::new_gd();
    /// let copy = resource.duplicate_resource(); // type Gd<Resource>
    /// assert_ne!(copy, resource);
    ///
    /// resource.free();
    /// copy.free();
    /// ```
    pub fn duplicate_resource(&self) -> Gd<T> {
        self.duplicate_resource_ex().done()
    }

    /// Duplicates this resource with a fluent builder API for fine-grained control.
    ///
    /// By default, this performs a shallow copy where all subresources are shared with the original,
    /// just like [`duplicate_resource()`][Self::duplicate_resource].
    ///
    /// You can change this behavior with:
    /// - [`deep()`][ExDuplicateResource::deep]: Deep copy using the legacy `duplicate(true)` API.
    /// - [`deep_subresources(mode)`][ExDuplicateResource::deep_subresources]: Deep copy with fine-grained control via [`DeepDuplicateMode`].
    ///
    /// This function can be used polymorphically: duplicating `Gd<Resource>` pointing to a dynamic type duplicates the concrete type.
    ///
    /// # Panics
    /// Panics if duplication fails. Likely causes:
    /// - The **dynamic** (runtime) type of the resource is not default-constructible. For example, if a `Gd<Resource>`
    ///   actually points to an instance of `MyResource` (inheriting `Resource`), and `MyResource` has no `init` or
    ///   uses `#[class(no_init)]`, then duplication will fail.
    ///
    /// To avoid panics, use [`ExDuplicateResource::try_done()`], [`ExDuplicateResource::try_deep()`], or
    /// [`ExDuplicateResource::try_deep_subresources()`].
    ///
    /// # Example
    /// ```no_run
    /// use godot::prelude::*;
    /// use godot::classes::resource::DeepDuplicateMode;
    ///
    /// let resource = Resource::new_gd();
    ///
    /// // Shallow duplication (default).
    /// let shallow = resource.duplicate_resource();
    ///
    /// // Deep duplication using the legacy API.
    /// let deep = resource.duplicate_resource_ex().deep();
    ///
    /// // Deep duplication with fine-grained control.
    /// let deep_all = resource.duplicate_resource_ex()
    ///     .deep_subresources(DeepDuplicateMode::ALL);
    /// ```
    pub fn duplicate_resource_ex(&self) -> ExDuplicateResource<'_, T> {
        ExDuplicateResource::new(self)
    }
}

/// Builder for duplicating a resource with deep duplication control.
///
/// Created by [`Gd::duplicate_resource_ex()`].
/// See [`duplicate_resource_ex()`](Gd::duplicate_resource_ex) for complete documentation including
/// default behavior, panic conditions, and polymorphic duplication.
///
/// This builder provides several terminal methods:
/// - [`done()`][Self::done]: Shallow duplication (subresources shared).
/// - [`deep()`][Self::deep]: Deep copy using the legacy `Resource::duplicate(true)` API.
/// - [`deep_subresources(mode)`][Self::deep_subresources]: Deep copy with fine-grained control via [`DeepDuplicateMode`].
///
/// All terminal methods have `try_*` variants that return `Result<Gd<T>, ()>` instead of panicking.
#[must_use]
pub struct ExDuplicateResource<'a, T>
where
    T: Inherits<Resource>,
{
    resource: &'a Gd<T>,
}

impl<'a, T> ExDuplicateResource<'a, T>
where
    T: Inherits<Resource>,
{
    fn new(resource: &'a Gd<T>) -> Self {
        Self { resource }
    }

    /// Complete shallow duplication and return the duplicated resource.
    ///
    /// This performs a shallow copy where all subresources are shared (same as [`Gd::duplicate_resource()`]).
    ///
    /// # Panics
    /// On errors, see [`Gd::duplicate_resource_ex()`]. To check for results, use [`try_done()`][Self::try_done].
    pub fn done(self) -> Gd<T> {
        let type_name = T::class_id();
        self.try_done()
            .unwrap_or_else(|()| panic!("Failed to duplicate {}", type_name))
    }

    /// Complete shallow duplication and return the duplicated resource, or an error if it fails.
    ///
    /// See [`Gd::duplicate_resource_ex()`] for details.
    pub fn try_done(self) -> Result<Gd<T>, ()> {
        self.resource
            .upcast_ref::<Resource>()
            .duplicate_full(false)
            .ok_or(())?
            .try_cast::<T>()
            .map_err(|_| ())
    }

    /// Complete deep duplication and return the duplicated resource.
    ///
    /// Uses the legacy `Resource::duplicate(true)` API, which duplicates all subresources.
    /// For fine-grained control over which subresources are duplicated, use [`deep_subresources()`][Self::deep_subresources] instead.
    ///
    /// # Panics
    /// On errors, see [`Gd::duplicate_resource_ex()`]. To check for results, use [`try_deep()`][Self::try_deep].
    ///
    /// # Example
    /// ```no_run
    /// use godot::prelude::*;
    ///
    /// let resource = Resource::new_gd();
    /// let deep_copy = resource.duplicate_resource_ex().deep();
    /// ```
    pub fn deep(self) -> Gd<T> {
        let type_name = T::class_id();
        self.try_deep()
            .unwrap_or_else(|()| panic!("Failed to duplicate {}", type_name))
    }

    /// Complete deep duplication and return the duplicated resource, or an error if it fails.
    ///
    /// See [`Gd::duplicate_resource_ex()`] and [`deep()`][Self::deep] for details.
    pub fn try_deep(self) -> Result<Gd<T>, ()> {
        self.resource
            .upcast_ref::<Resource>()
            .duplicate_full(true)
            .ok_or(())?
            .try_cast::<T>()
            .map_err(|_| ())
    }

    /// Complete deep duplication with fine-grained control and return the duplicated resource.
    ///
    /// Uses the newer `Resource::duplicate_deep(mode)` API.
    ///
    /// # Modes
    ///
    /// - [`DeepDuplicateMode::NONE`]: Shallow copy, all subresources are shared (same as [`Gd::duplicate_resource()`]).
    /// - [`DeepDuplicateMode::INTERNAL`]: Duplicate internal subresources only.
    /// - [`DeepDuplicateMode::ALL`]: Duplicate all subresources, including those in `Array`/`Dictionary`.
    ///
    /// # Panics
    /// On errors, see [`Gd::duplicate_resource_ex()`]. To check for results, use [`try_deep_subresources()`][Self::try_deep_subresources].
    ///
    /// # Example
    /// ```no_run
    /// use godot::prelude::*;
    /// use godot::classes::resource::DeepDuplicateMode;
    ///
    /// let resource = Resource::new_gd();
    ///
    /// // Duplicate internal subresources only.
    /// let internal = resource.duplicate_resource_ex()
    ///     .deep_subresources(DeepDuplicateMode::INTERNAL);
    ///
    /// // Duplicate all subresources including those in Array/Dictionary.
    /// let all = resource.duplicate_resource_ex()
    ///     .deep_subresources(DeepDuplicateMode::ALL);
    /// ```
    pub fn deep_subresources(self, mode: DeepDuplicateMode) -> Gd<T> {
        let type_name = T::class_id();
        self.try_deep_subresources(mode)
            .unwrap_or_else(|()| panic!("Failed to duplicate {}", type_name))
    }

    /// Complete deep duplication with fine-grained control, or return an error if it fails.
    ///
    /// See [`Gd::duplicate_resource_ex()`] and [`deep_subresources()`][Self::deep_subresources] for details.
    pub fn try_deep_subresources(self, mode: DeepDuplicateMode) -> Result<Gd<T>, ()> {
        self.resource
            .upcast_ref::<Resource>()
            .duplicate_deep_full(mode)
            .ok_or(())?
            .try_cast::<T>()
            .map_err(|_| ())
    }
}
