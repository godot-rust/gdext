/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::{HashMap, HashSet};

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

use crate::util::{bail, ident, KvParser, ListParser};
use crate::ParseResult;

pub struct FieldExport {
    pub export_type: ExportType,
    pub span: Span,
}

impl FieldExport {
    pub(crate) fn new_from_kv(parser: &mut KvParser) -> ParseResult<Self> {
        let span = parser.span();
        let export_type = ExportType::new_from_kv(parser)?;
        Ok(Self { export_type, span })
    }

    pub fn to_export_hint(&self) -> Option<TokenStream> {
        self.export_type.to_export_hint()
    }

    pub fn to_export_usage(&self) -> Option<Ident> {
        self.export_type.to_export_usage()
    }
}

/// Store info from `#[export]` attribute.
pub enum ExportType {
    /// ### GDScript annotations
    /// - `@export`
    ///
    /// ### Property hints
    /// - `NONE` (usually)
    ///
    /// Can become other property hints, depends on context.
    Default,

    /// ### GDScript annotations
    /// - `@export_storage`
    ///
    /// ### Property hints
    /// - `NONE`
    ///
    /// ### Property usage
    /// - `STORAGE`
    ///
    /// This is used to indicate that the property should be exported
    /// but should not be visible in the editor. Therefore, it does not
    /// have a property hint, but uses the `STORAGE` property usage.
    Storage,

    /// ### GDScript annotations
    /// - `@export_range`
    ///
    /// ### Property hints
    /// - `RANGE`
    Range {
        min: TokenStream,
        max: TokenStream,
        step: TokenStream,
        or_greater: bool,
        or_less: bool,
        exp: bool,
        radians_as_degrees: bool,
        degrees: bool,
        hide_slider: bool,
        suffix: Option<TokenStream>,
    },

    /// ### GDScript annotations
    /// - `@export_enum`
    ///
    /// ### Property hints
    /// - `ENUM`
    Enum { variants: Vec<ValueWithKey> },

    /// ### GDScript annotations
    /// - `@export_exp_easing`
    ///
    /// ### Property hints
    /// - `EXP_EASING`
    ExpEasing {
        attenuation: bool,
        positive_only: bool,
    },

    /// ### GDScript annotations
    /// - `@export_flags`
    ///
    /// ### Property hints
    /// - `FLAGS`
    Flags { bits: Vec<ValueWithKey> },

    /// ### GDScript annotations
    /// - `@export_flags_2d_physics`
    /// - `@export_flags_2d_render`
    /// - `@export_flags_2d_navigation`
    /// - `@export_flags_3d_physics`
    /// - `@export_flags_3d_render`
    /// - `@export_flags_3d_navigation`
    ///
    /// ### Property hints
    /// - `LAYERS_2D_PHYSICS`
    /// - `LAYERS_2D_RENDER`
    /// - `LAYERS_2D_NAVIGATION`
    /// - `LAYERS_3D_PHYSICS`
    /// - `LAYERS_3D_RENDER`
    /// - `LAYERS_3D_NAVIGATION`
    Layers {
        dimension: LayerDimension,
        kind: LayerKind,
    },

    /// ### GDScript annotations
    /// - `@export_file`
    /// - `@export_global_file`
    /// - `@export_dir`
    /// - `@export_global_dir`
    ///
    /// ### Property hints
    /// - `FILE`
    /// - `GLOBAL_FILE`
    /// - `DIR`
    /// - `GLOBAL_DIR`
    File { global: bool, kind: FileKind },

    /// ### GDScript annotations
    /// - `@export_multiline`
    ///
    /// ### Property hints
    /// - `MULTILINE_TEXT`
    Multiline,

    /// ### GDScript annotations
    /// - `@export_placeholder`
    ///
    /// ### Property hints
    /// - `PLACEHOLDER_TEXT`
    PlaceholderText { placeholder: TokenStream },

    /// ### GDScript annotations
    /// - `@export_color_no_alpha`
    ///
    /// ### Property hints
    /// - `COLOR_NO_ALPHA`
    ColorNoAlpha,
}

impl ExportType {
    /// Parse an `#[export(...)]` attribute.
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
        if parser.handle_alone("storage")? {
            return Self::new_storage();
        }

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

        Ok(Self::Default)
    }

    fn new_storage() -> ParseResult<Self> {
        Ok(Self::Storage)
    }

    fn new_range_list(mut parser: ListParser) -> ParseResult<Self> {
        const FLAG_OPTIONS: [&str; 7] = [
            "or_greater",
            "or_less",
            "exp",
            "radians_as_degrees",
            "radians", // Godot deprecated this key since 4.2, in favor of `radians_as_degrees`.
            "degrees",
            "hide_slider",
        ];
        const KV_OPTIONS: [&str; 1] = ["suffix"];

        let min = parser.next_expr()?;
        let max = parser.next_expr()?;
        // If there is a next element, and it is a literal, we take its tokens directly.
        let step = if parser.peek().is_some_and(|kv| kv.as_literal().is_ok()) {
            let value = parser
                .next_expr()
                .expect("already guaranteed there was a TokenTree to parse");
            quote! { Some(#value) }
        } else {
            quote! { None }
        };

        let mut flags = HashSet::<String>::new();
        let mut kvs = HashMap::<String, TokenStream>::new();

        loop {
            let key_maybe_value =
                parser.next_allowed_key_optional_value(&FLAG_OPTIONS, &KV_OPTIONS)?;
            match key_maybe_value {
                Some((ident, None)) => {
                    if ident == "radians" {
                        return bail!(
                            &ident,
                            "#[export(range = (...))]: `radians` is broken in Godot and superseded by `radians_as_degrees`.\n\
                            See https://github.com/godotengine/godot/pull/82195 for details."
                        );
                    }

                    flags.insert(ident.to_string());
                }
                Some((ident, Some(value))) => {
                    kvs.insert(ident.to_string(), value.expr()?);
                }
                None => break,
            }
        }

        parser.finish()?;

        Ok(Self::Range {
            min,
            max,
            step,
            or_greater: flags.contains("or_greater"),
            or_less: flags.contains("or_less"),
            exp: flags.contains("exp"),
            radians_as_degrees: flags.contains("radians_as_degrees"),
            degrees: flags.contains("degrees"),
            hide_slider: flags.contains("hide_slider"),
            suffix: kvs.get("suffix").cloned(),
        })
    }

    fn new_enum_export(mut parser: ListParser) -> ParseResult<Self> {
        let mut variants = Vec::new();

        while let Some((key, kv)) = parser.next_key_optional_value()? {
            let integer = kv.map(|kv| kv.expr()).transpose()?;

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

        while let Some(option) = parser.next_allowed_ident(&ALLOWED_OPTIONS[..])? {
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
            let integer = kv.map(|kv| kv.expr()).transpose()?;

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
        Some(quote! {
            ::godot::register::property::export_info_functions::$function_name($($tt)*)
        })
    };

    // Passes in a previously declared local `type FieldType = ...` as first generic argument.
    // Doesn't work if function takes other generic arguments -- in that case it could be converted to a Type<...> parameter.
    ($function_name:ident < T > ($($tt:tt)*)) => {
        Some(quote! {
            ::godot::register::property::export_info_functions::$function_name::<FieldType>($($tt)*)
        })
    };
}

impl ExportType {
    pub fn to_export_hint(&self) -> Option<TokenStream> {
        match self {
            Self::Default => None,

            Self::Storage => quote_export_func! { export_storage() },

            Self::Range {
                min,
                max,
                step,
                or_greater,
                or_less,
                exp,
                radians_as_degrees,
                degrees,
                hide_slider,
                suffix,
            } => {
                let suffix = if suffix.is_some() {
                    quote! { Some(#suffix.to_string()) }
                } else {
                    quote! { None }
                };
                let export_func = quote_export_func! {
                    export_range(#min, #max, #step, #or_greater, #or_less, #exp, #radians_as_degrees, #degrees, #hide_slider, #suffix)
                }?;
                Some(export_func)
            }

            Self::Enum { variants } => {
                let variants = variants.iter().map(ValueWithKey::to_tuple_expression);

                quote_export_func! {
                    export_enum(&[#(#variants),*])
                }
            }

            Self::ExpEasing {
                attenuation,
                positive_only,
            } => quote_export_func! {
                    export_exp_easing(#attenuation, #positive_only)
            },

            Self::Flags { bits } => {
                let bits = bits.iter().map(ValueWithKey::to_tuple_expression);

                quote_export_func! {
                    export_flags(&[#(#bits),*])
                }
            }

            Self::Layers {
                dimension: LayerDimension::_2d,
                kind: LayerKind::Physics,
            } => quote_export_func! { export_flags_2d_physics() },

            Self::Layers {
                dimension: LayerDimension::_2d,
                kind: LayerKind::Render,
            } => quote_export_func! { export_flags_2d_render() },

            Self::Layers {
                dimension: LayerDimension::_2d,
                kind: LayerKind::Navigation,
            } => quote_export_func! { export_flags_2d_navigation() },

            Self::Layers {
                dimension: LayerDimension::_3d,
                kind: LayerKind::Physics,
            } => quote_export_func! { export_flags_3d_physics() },

            Self::Layers {
                dimension: LayerDimension::_3d,
                kind: LayerKind::Render,
            } => quote_export_func! { export_flags_3d_render() },

            Self::Layers {
                dimension: LayerDimension::_3d,
                kind: LayerKind::Navigation,
            } => quote_export_func! { export_flags_3d_navigation() },

            Self::File {
                global,
                kind: FileKind::Dir,
            } => {
                let filter = quote! { "" };
                quote_export_func! { export_file_or_dir<T>(false, #global, #filter) }
            }

            Self::File {
                global,
                kind: FileKind::File { filter },
            } => {
                let filter = filter.clone().unwrap_or(quote! { "" });
                quote_export_func! { export_file_or_dir<T>(true, #global, #filter) }
            }

            Self::Multiline => quote_export_func! { export_multiline() },

            Self::PlaceholderText { placeholder } => quote_export_func! {
                export_placeholder(#placeholder)
            },

            Self::ColorNoAlpha => quote_export_func! { export_color_no_alpha() },
        }
    }

    /// Returns a `PropertyUsageFlags` identifier if this export type has a _usage_.
    pub fn to_export_usage(&self) -> Option<Ident> {
        match self {
            Self::Storage => Some(ident("STORAGE")),
            _ => None,
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
