/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, TokenStream};
use quote::quote;
use std::collections::HashSet;

use crate::class::FieldHint;
use crate::util::{KvParser, ListParser};
use crate::ParseResult;

/// Store info from `#[export]` attribute.
pub enum FieldExport {
    /// ### GDScript Annotations
    /// - `@export`
    ///
    /// ### Property Hints
    /// - `PROPERTY_HINT_NONE` (usually)
    ///
    /// Can become other property hints, depends on context.
    Default,

    /// ### GDScript Annotations
    /// - `@export_range`
    ///
    /// ### Property Hints
    /// - `PROPERTY_HINT_RANGE`
    Range {
        min: TokenStream,
        max: TokenStream,
        step: TokenStream,
        or_greater: bool,
        or_less: bool,
        exp: bool,
        radians: bool,
        degrees: bool,
        hide_slider: bool,
    },

    /// ### GDScript Annotations
    /// - `@export_enum`
    ///
    /// ### Property Hints
    /// - `PROPERTY_HINT_ENUM`
    Enum { variants: Vec<ValueWithKey> },

    /// ### GDScript Annotations
    /// - `@export_exp_easing`
    ///
    /// ### Property Hints
    /// - `PROPERTY_HINT_EXP_EASING`
    ExpEasing {
        attenuation: bool,
        positive_only: bool,
    },

    /// ### GDScript Annotations
    /// - `@export_flags`
    ///
    /// ### Property Hints
    /// - `PROPERTY_HINT_FLAGS`
    Flags { bits: Vec<ValueWithKey> },

    /// ### GDScript Annotations
    /// - `@export_flags_2d_physics`
    /// - `@export_flags_2d_render`
    /// - `@export_flags_2d_navigation`
    /// - `@export_flags_3d_physics`
    /// - `@export_flags_3d_render`
    /// - `@export_flags_3d_navigation`
    ///
    /// ### Property Hints
    /// - `PROPERTY_HINT_LAYERS_2D_PHYSICS`
    /// - `PROPERTY_HINT_LAYERS_2D_RENDER`
    /// - `PROPERTY_HINT_LAYERS_2D_NAVIGATION`
    /// - `PROPERTY_HINT_LAYERS_3D_PHYSICS`
    /// - `PROPERTY_HINT_LAYERS_3D_RENDER`
    /// - `PROPERTY_HINT_LAYERS_3D_NAVIGATION`
    Layers {
        dimension: LayerDimension,
        kind: LayerKind,
    },

    /// ### GDScript Annotations
    /// - `@export_file`
    /// - `@export_global_file`
    /// - `@export_dir`
    /// - `@export_global_dir`
    ///
    /// ### Property Hints
    /// - `PROPERTY_HINT_FILE`
    /// - `PROPERTY_HINT_GLOBAL_FILE`
    /// - `PROPERTY_HINT_DIR`
    /// - `PROPERTY_HINT_GLOBAL_DIR`
    File { global: bool, kind: FileKind },

    /// ### GDScript Annotations
    /// - `@export_multiline`
    ///
    /// ### Property Hints
    /// - `PROPERTY_HINT_MULTILINE_TEXT`
    Multiline,

    /// ### GDScript Annotations
    /// - `@export_placeholder`
    ///
    /// ### Property Hints
    /// - `PROPERTY_HINT_PLACEHOLDER_TEXT`
    PlaceholderText { placeholder: TokenStream },

    /// ### GDScript Annotations
    /// - `@export_color_no_alpha`
    ///
    /// ### Property Hints
    /// - `PROPERTY_HINT_COLOR_NO_ALPHA`
    ColorNoAlpha,
}

impl FieldExport {
    /// Parse an `#[export(..)]` attribute.
    ///
    /// The translation from GDScript annotations to rust attributes is given by:
    ///
    /// - `@export` becomes `#[export]`
    /// - `@export_{name}` becomes `#[export(name)]`
    /// - `@export_{name}(elem1, ...)` becomes `#[export(name = (elem1, ...))]`
    /// - `@export_{flags/enum}("elem1", "elem2:key2", ...)`
    ///   becomes
    ///   `#[export(flags/enum = (elem1, elem2 = key2, ...))]`
    pub(crate) fn new_from_kv(parser: &mut KvParser) -> ParseResult<Self> {
        if let Some(list_parser) = parser.handle_list("range")? {
            return Self::new_range_list(list_parser);
        }

        if let Some(list_parser) = parser.handle_list("enum")? {
            return Self::new_enum_export(list_parser);
        }

        if let Some(list_parser) = parser.handle_list("exp_easing")? {
            return Self::new_exp_easing(list_parser);
        }

        if let Some(list_parser) = parser.handle_list("flags")? {
            return Self::new_flags(list_parser);
        }

        if parser.handle_alone("flags_2d_render")? {
            return Ok(Self::Layers {
                dimension: LayerDimension::_2d,
                kind: LayerKind::Render,
            });
        }

        if parser.handle_alone("flags_2d_physics")? {
            return Ok(Self::Layers {
                dimension: LayerDimension::_2d,
                kind: LayerKind::Physics,
            });
        }

        if parser.handle_alone("flags_2d_navigation")? {
            return Ok(Self::Layers {
                dimension: LayerDimension::_2d,
                kind: LayerKind::Navigation,
            });
        }

        if parser.handle_alone("flags_3d_render")? {
            return Ok(Self::Layers {
                dimension: LayerDimension::_3d,
                kind: LayerKind::Render,
            });
        }

        if parser.handle_alone("flags_3d_physics")? {
            return Ok(Self::Layers {
                dimension: LayerDimension::_3d,
                kind: LayerKind::Physics,
            });
        }

        if parser.handle_alone("flags_3d_navigation")? {
            return Ok(Self::Layers {
                dimension: LayerDimension::_3d,
                kind: LayerKind::Navigation,
            });
        }

        match parser.handle_any("file") {
            Some(None) => {
                return Ok(Self::File {
                    global: false,
                    kind: FileKind::File { filter: None },
                })
            }
            Some(Some(kv)) => {
                return Ok(Self::File {
                    global: false,
                    kind: FileKind::File {
                        filter: Some(kv.expr()?),
                    },
                })
            }
            None => (),
        }

        match parser.handle_any("global_file") {
            Some(None) => {
                return Ok(Self::File {
                    global: true,
                    kind: FileKind::File { filter: None },
                })
            }
            Some(Some(kv)) => {
                return Ok(Self::File {
                    global: true,
                    kind: FileKind::File {
                        filter: Some(kv.expr()?),
                    },
                })
            }
            None => (),
        }

        if parser.handle_alone("dir")? {
            return Ok(Self::File {
                global: false,
                kind: FileKind::Dir,
            });
        }

        if parser.handle_alone("global_dir")? {
            return Ok(Self::File {
                global: true,
                kind: FileKind::Dir,
            });
        }

        if parser.handle_alone("multiline")? {
            return Ok(Self::Multiline);
        }

        if let Some(placeholder) = parser.handle_expr("placeholder")? {
            return Ok(Self::PlaceholderText { placeholder });
        }

        if parser.handle_alone("color_no_alpha")? {
            return Ok(Self::ColorNoAlpha);
        }

        Ok(FieldExport::Default)
    }

    fn new_range_list(mut parser: ListParser) -> ParseResult<FieldExport> {
        const ALLOWED_OPTIONS: [&str; 6] = [
            "or_greater",
            "or_less",
            "exp",
            "radians",
            "degrees",
            "hide_slider",
        ];

        let min = parser.next_expr()?;
        let max = parser.next_expr()?;
        // TODO: During parser refactor, try to remove the need for `is_next_ident` there. Currently needed only for this functionality.
        // See discussion for rationale here: https://github.com/godot-rust/gdext/pull/484#pullrequestreview-1738612069
        let step = match parser.is_next_ident() {
            Some(false) => {
                let value = parser.next_expr()?;
                quote! { Some(#value) }
            }
            _ => quote! { None },
        };

        let mut options = HashSet::new();

        while let Some(option) = parser.next_any_ident(&ALLOWED_OPTIONS[..])? {
            options.insert(option.to_string());
        }

        parser.finish()?;

        Ok(FieldExport::Range {
            min,
            max,
            step,
            or_greater: options.contains("or_greater"),
            or_less: options.contains("or_less"),
            exp: options.contains("exp"),
            radians: options.contains("radians"),
            degrees: options.contains("degrees"),
            hide_slider: options.contains("hide_slider"),
        })
    }

    fn new_enum_export(mut parser: ListParser) -> ParseResult<Self> {
        let mut variants = Vec::new();

        while let Some((key, kv)) = parser.next_key_optional_value()? {
            let integer = match kv {
                Some(kv) => Some(kv.expr()?),
                None => None,
            };

            variants.push(ValueWithKey {
                key,
                value: integer,
            });
        }

        parser.finish()?;

        Ok(Self::Enum { variants })
    }

    fn new_exp_easing(mut parser: ListParser) -> ParseResult<Self> {
        const ALLOWED_OPTIONS: [&str; 2] = ["attenuation", "positive_only"];

        let mut options = HashSet::new();

        while let Some(option) = parser.next_any_ident(&ALLOWED_OPTIONS[..])? {
            options.insert(option.to_string());
        }

        parser.finish()?;

        Ok(Self::ExpEasing {
            attenuation: options.contains("attenuation"),
            positive_only: options.contains("positive_only"),
        })
    }

    fn new_flags(mut parser: ListParser) -> ParseResult<Self> {
        let mut bits = Vec::new();

        while let Some((key, kv)) = parser.next_key_optional_value()? {
            let integer = match kv {
                Some(kv) => Some(kv.expr()?),
                None => None,
            };

            bits.push(ValueWithKey {
                key,
                value: integer,
            });
        }

        parser.finish()?;

        Ok(Self::Flags { bits })
    }
}

macro_rules! quote_export_func {
    ($function_name:ident($($tt:tt)*)) => {
        FieldHint::HintFromExportFunction(quote! {
            ::godot::bind::property::export_info_functions::$function_name($($tt)*)
        })
    }
}

impl FieldExport {
    pub fn to_field_hint(&self) -> FieldHint {
        match self {
            FieldExport::Default => FieldHint::Inferred,

            FieldExport::Range {
                min,
                max,
                step,
                or_greater,
                or_less,
                exp,
                radians,
                degrees,
                hide_slider,
            } => quote_export_func! {
                export_range(#min, #max, #step, #or_greater, #or_less, #exp, #radians, #degrees, #hide_slider)
            },

            FieldExport::Enum { variants } => {
                let variants = variants.iter().map(ValueWithKey::to_tuple_expression);

                quote_export_func! {
                    export_enum(&[#(#variants),*])
                }
            }

            FieldExport::ExpEasing {
                attenuation,
                positive_only,
            } => quote_export_func! {
                    export_exp_easing(#attenuation, #positive_only)
            },

            FieldExport::Flags { bits } => {
                let bits = bits.iter().map(ValueWithKey::to_tuple_expression);

                quote_export_func! {
                    export_flags(&[#(#bits),*])
                }
            }

            FieldExport::Layers {
                dimension: LayerDimension::_2d,
                kind: LayerKind::Physics,
            } => quote_export_func! { export_flags_2d_physics() },

            FieldExport::Layers {
                dimension: LayerDimension::_2d,
                kind: LayerKind::Render,
            } => quote_export_func! { export_flags_2d_render() },

            FieldExport::Layers {
                dimension: LayerDimension::_2d,
                kind: LayerKind::Navigation,
            } => quote_export_func! { export_flags_2d_navigation() },

            FieldExport::Layers {
                dimension: LayerDimension::_3d,
                kind: LayerKind::Physics,
            } => quote_export_func! { export_flags_3d_physics() },

            FieldExport::Layers {
                dimension: LayerDimension::_3d,
                kind: LayerKind::Render,
            } => quote_export_func! { export_flags_3d_render() },

            FieldExport::Layers {
                dimension: LayerDimension::_3d,
                kind: LayerKind::Navigation,
            } => quote_export_func! { export_flags_3d_navigation() },

            FieldExport::File {
                global: false,
                kind: FileKind::Dir,
            } => quote_export_func! { export_dir() },

            FieldExport::File {
                global: true,
                kind: FileKind::Dir,
            } => quote_export_func! { export_global_dir() },

            FieldExport::File {
                global,
                kind: FileKind::File { filter },
            } => {
                let filter = filter.clone().unwrap_or(quote! { "" });

                quote_export_func! { export_file_inner(#global, #filter) }
            }

            FieldExport::Multiline => quote_export_func! { export_multiline() },

            FieldExport::PlaceholderText { placeholder } => quote_export_func! {
                export_placeholder(#placeholder)
            },
            FieldExport::ColorNoAlpha => quote_export_func! { export_color_no_alpha() },
        }
    }
}

/// The dimension of a `@export_flags_{dimension}_{layer}` annotation.
pub enum LayerDimension {
    _2d,
    _3d,
}

/// The layer of a `@export_flags_{dimension}_{layer}` annotation.
pub enum LayerKind {
    Render,
    Physics,
    Navigation,
}

/// Whether we're dealing with a `@export_dir` or `@export_file` annotation.
pub enum FileKind {
    File { filter: Option<TokenStream> },
    Dir,
}

/// A `key = value` pair used for enums and bitflags.
///
/// `key` must be an identifier, and `value` some tokenstream that can be coerced into the appropriate
/// integer type for the context. For enums that is i64, and for bitflags that is u32.
///
/// `key = value` becomes `key:value` in the hint_string.
#[derive(Clone)]
pub struct ValueWithKey {
    key: Ident,
    value: Option<TokenStream>,
}

impl ValueWithKey {
    /// Create an expression like `(key, value)` that can be passed to the relevant export info function.
    pub fn to_tuple_expression(&self) -> TokenStream {
        let ValueWithKey { key, value } = self;
        let key = key.to_string();

        match value {
            Some(value) => quote! {
                (#key, Some(#value))
            },
            None => quote! {
                (#key, None)
            },
        }
    }
}
