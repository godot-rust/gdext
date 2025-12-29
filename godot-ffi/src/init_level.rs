/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

/// Step in the Godot initialization process.
///
/// Godot's initialization and deinitialization processes are split into multiple stages, like a stack. At each level,
/// a different amount of engine functionality is available. Deinitialization happens in reverse order.
///
/// See also:
// Explicit HTML links because this is re-exported in godot::init, and we can't document a `use` statement.
/// - [`InitStage`](enum.InitStage.html): all levels + main loop.
/// - [`ExtensionLibrary::on_stage_init()`](trait.ExtensionLibrary.html#method.on_stage_init)
/// - [`ExtensionLibrary::on_stage_deinit()`](trait.ExtensionLibrary.html#method.on_stage_deinit)
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum InitLevel {
    /// First level loaded by Godot. Builtin types are available, classes are not.
    Core,

    /// Second level loaded by Godot. Only server classes and builtins are available.
    Servers,

    /// Third level loaded by Godot. Most classes are available.
    Scene,

    /// Fourth level loaded by Godot, only in the editor. All classes are available.
    Editor,
}

impl InitLevel {
    #[doc(hidden)]
    pub fn from_sys(level: crate::GDExtensionInitializationLevel) -> Self {
        match level {
            crate::GDEXTENSION_INITIALIZATION_CORE => Self::Core,
            crate::GDEXTENSION_INITIALIZATION_SERVERS => Self::Servers,
            crate::GDEXTENSION_INITIALIZATION_SCENE => Self::Scene,
            crate::GDEXTENSION_INITIALIZATION_EDITOR => Self::Editor,
            _ => {
                eprintln!("WARNING: unknown initialization level {level}");
                Self::Scene
            }
        }
    }

    #[doc(hidden)]
    pub fn to_sys(self) -> crate::GDExtensionInitializationLevel {
        match self {
            Self::Core => crate::GDEXTENSION_INITIALIZATION_CORE,
            Self::Servers => crate::GDEXTENSION_INITIALIZATION_SERVERS,
            Self::Scene => crate::GDEXTENSION_INITIALIZATION_SCENE,
            Self::Editor => crate::GDEXTENSION_INITIALIZATION_EDITOR,
        }
    }

    /// Convert this initialization level to an initialization stage.
    pub fn to_stage(self) -> InitStage {
        match self {
            Self::Core => InitStage::Core,
            Self::Servers => InitStage::Servers,
            Self::Scene => InitStage::Scene,
            Self::Editor => InitStage::Editor,
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Extended step in the initialization process, including both init-levels and the main loop.
///
/// This enum extends [`InitLevel`] with a `MainLoop` variant, representing the fully initialized state of Godot
/// after all initialization levels have been loaded and before any deinitialization begins.
///
/// During initialization, stages are loaded in order: `Core` → `Servers` → `Scene` → `Editor` (if in editor) → `MainLoop`.  \
/// During deinitialization, stages are unloaded in reverse order.
///
/// See also:
/// - [`InitLevel`](enum.InitLevel.html): only levels, without `MainLoop`.
/// - [`ExtensionLibrary::on_stage_init()`](trait.ExtensionLibrary.html#method.on_stage_init)
/// - [`ExtensionLibrary::on_stage_deinit()`](trait.ExtensionLibrary.html#method.on_stage_deinit)
/// - [`ExtensionLibrary::on_main_loop_frame()`](trait.ExtensionLibrary.html#method.on_main_loop_frame)
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[non_exhaustive]
pub enum InitStage {
    /// First level loaded by Godot. Builtin types are available, classes are not.
    Core,

    /// Second level loaded by Godot. Only server classes and builtins are available.
    Servers,

    /// Third level loaded by Godot. Most classes are available.
    Scene,

    /// Fourth level loaded by Godot, only in the editor. All classes are available.
    Editor,

    /// The main loop stage, representing the fully initialized state of Godot.
    ///
    /// This variant is only available in Godot 4.5+. In earlier versions, it will never be passed to callbacks.
    /// It is however unconditionally available, to avoid "infecting" user code with `#[cfg]`s.
    MainLoop,
}

impl InitStage {
    /// Try to convert this initialization stage to an initialization level.
    ///
    /// Returns `None` for [`InitStage::MainLoop`], as it doesn't correspond to a Godot initialization level.
    pub fn try_to_level(self) -> Option<InitLevel> {
        match self {
            Self::Core => Some(InitLevel::Core),
            Self::Servers => Some(InitLevel::Servers),
            Self::Scene => Some(InitLevel::Scene),
            Self::Editor => Some(InitLevel::Editor),
            Self::MainLoop => None,
        }
    }
}
