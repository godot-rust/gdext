use crate as sys;

/// Dispatch at runtime between Godot 4.0 legacy and 4.1+ APIs.
pub(crate) trait CompatVersion {
    /// Return whether a gdext version compiled against 4.1+ GDExtension is invoked with an entry point using the legacy calling convention.
    ///
    /// This can happen in two cases:
    /// * The .gdextension file's `[configuration]` section does not contain a `compatibility_minimum = 4.1` statement.
    /// * gdext was compiled against a 4.1+ Godot version, but at runtime the library is loaded from a 4.0.x version.
    fn is_legacy_used_in_modern(&self) -> bool;

    /// Return version dynamically passed via `gdextension_interface.h` file.
    fn runtime_version(&self) -> sys::GDExtensionGodotVersion;

    /// Return the interface, either as-is from the header (legacy) or code-generated (modern API).
    fn load_interface(&self) -> sys::GDExtensionInterface;
}
