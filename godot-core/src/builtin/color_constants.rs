/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::Color;

/// Godot's predefined colors
///
/// This [visual cheat sheet](https://raw.githubusercontent.com/godotengine/godot-docs/master/img/color_constants.png)
/// shows how the colors look.
impl Color {
    /// Transparent black.
    ///
    /// This color is not provided by Godot, so [`Color::from_string("TRANSPARENT_BLACK")`](Color::from_string) will be `None`.
    pub const TRANSPARENT_BLACK: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.0);
    /// Transparent white.
    ///
    /// This color is not provided by Godot, so [`Color::from_string("TRANSPARENT_WHITE")`](Color::from_string) will be `None`.
    /// Use `Color::from_string("TRANSPARENT")` instead.
    ///
    /// _Godot equivalent: `Color.TRANSPARENT`_
    pub const TRANSPARENT_WHITE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.0);
    /// Black color. This is the [default](Color::default) value.
    pub const BLACK: Color = Color::from_rgba(0.0, 0.0, 0.0, 1.0);
    pub const WHITE: Color = Color::from_rgba(1.0, 1.0, 1.0, 1.0);
    pub const ALICE_BLUE: Color = Color::from_rgba(0.941176, 0.972549, 1.0, 1.0);
    pub const ANTIQUE_WHITE: Color = Color::from_rgba(0.980392, 0.921569, 0.843137, 1.0);
    pub const AQUA: Color = Color::from_rgba(0.0, 1.0, 1.0, 1.0);
    pub const AQUAMARINE: Color = Color::from_rgba(0.498039, 1.0, 0.831373, 1.0);
    pub const AZURE: Color = Color::from_rgba(0.941176, 1.0, 1.0, 1.0);
    pub const BEIGE: Color = Color::from_rgba(0.960784, 0.960784, 0.862745, 1.0);
    pub const BISQUE: Color = Color::from_rgba(1.0, 0.894118, 0.768627, 1.0);
    pub const BLANCHED_ALMOND: Color = Color::from_rgba(1.0, 0.921569, 0.803922, 1.0);
    pub const BLUE: Color = Color::from_rgba(0.0, 0.0, 1.0, 1.0);
    pub const BLUE_VIOLET: Color = Color::from_rgba(0.541176, 0.168627, 0.886275, 1.0);
    pub const BROWN: Color = Color::from_rgba(0.647059, 0.164706, 0.164706, 1.0);
    pub const BURLYWOOD: Color = Color::from_rgba(0.870588, 0.721569, 0.529412, 1.0);
    pub const CADET_BLUE: Color = Color::from_rgba(0.372549, 0.619608, 0.627451, 1.0);
    pub const CHARTREUSE: Color = Color::from_rgba(0.498039, 1.0, 0.0, 1.0);
    pub const CHOCOLATE: Color = Color::from_rgba(0.823529, 0.411765, 0.117647, 1.0);
    pub const CORAL: Color = Color::from_rgba(1.0, 0.498039, 0.313726, 1.0);
    pub const CORNFLOWER_BLUE: Color = Color::from_rgba(0.392157, 0.584314, 0.929412, 1.0);
    pub const CORNSILK: Color = Color::from_rgba(1.0, 0.972549, 0.862745, 1.0);
    pub const CRIMSON: Color = Color::from_rgba(0.862745, 0.0784314, 0.235294, 1.0);
    pub const CYAN: Color = Color::from_rgba(0.0, 1.0, 1.0, 1.0);
    pub const DARK_BLUE: Color = Color::from_rgba(0.0, 0.0, 0.545098, 1.0);
    pub const DARK_CYAN: Color = Color::from_rgba(0.0, 0.545098, 0.545098, 1.0);
    pub const DARK_GOLDENROD: Color = Color::from_rgba(0.721569, 0.52549, 0.0431373, 1.0);
    pub const DARK_GRAY: Color = Color::from_rgba(0.662745, 0.662745, 0.662745, 1.0);
    pub const DARK_GREEN: Color = Color::from_rgba(0.0, 0.392157, 0.0, 1.0);
    pub const DARK_KHAKI: Color = Color::from_rgba(0.741176, 0.717647, 0.419608, 1.0);
    pub const DARK_MAGENTA: Color = Color::from_rgba(0.545098, 0.0, 0.545098, 1.0);
    pub const DARK_OLIVE_GREEN: Color = Color::from_rgba(0.333333, 0.419608, 0.184314, 1.0);
    pub const DARK_ORANGE: Color = Color::from_rgba(1.0, 0.54902, 0.0, 1.0);
    pub const DARK_ORCHID: Color = Color::from_rgba(0.6, 0.196078, 0.8, 1.0);
    pub const DARK_RED: Color = Color::from_rgba(0.545098, 0.0, 0.0, 1.0);
    pub const DARK_SALMON: Color = Color::from_rgba(0.913725, 0.588235, 0.478431, 1.0);
    pub const DARK_SEA_GREEN: Color = Color::from_rgba(0.560784, 0.737255, 0.560784, 1.0);
    pub const DARK_SLATE_BLUE: Color = Color::from_rgba(0.282353, 0.239216, 0.545098, 1.0);
    pub const DARK_SLATE_GRAY: Color = Color::from_rgba(0.184314, 0.309804, 0.309804, 1.0);
    pub const DARK_TURQUOISE: Color = Color::from_rgba(0.0, 0.807843, 0.819608, 1.0);
    pub const DARK_VIOLET: Color = Color::from_rgba(0.580392, 0.0, 0.827451, 1.0);
    pub const DEEP_PINK: Color = Color::from_rgba(1.0, 0.0784314, 0.576471, 1.0);
    pub const DEEP_SKY_BLUE: Color = Color::from_rgba(0.0, 0.74902, 1.0, 1.0);
    pub const DIM_GRAY: Color = Color::from_rgba(0.411765, 0.411765, 0.411765, 1.0);
    pub const DODGER_BLUE: Color = Color::from_rgba(0.117647, 0.564706, 1.0, 1.0);
    pub const FIREBRICK: Color = Color::from_rgba(0.698039, 0.133333, 0.133333, 1.0);
    pub const FLORAL_WHITE: Color = Color::from_rgba(1.0, 0.980392, 0.941176, 1.0);
    pub const FOREST_GREEN: Color = Color::from_rgba(0.133333, 0.545098, 0.133333, 1.0);
    pub const FUCHSIA: Color = Color::from_rgba(1.0, 0.0, 1.0, 1.0);
    pub const GAINSBORO: Color = Color::from_rgba(0.862745, 0.862745, 0.862745, 1.0);
    pub const GHOST_WHITE: Color = Color::from_rgba(0.972549, 0.972549, 1.0, 1.0);
    pub const GOLD: Color = Color::from_rgba(1.0, 0.843137, 0.0, 1.0);
    pub const GOLDENROD: Color = Color::from_rgba(0.854902, 0.647059, 0.12549, 1.0);
    pub const GRAY: Color = Color::from_rgba(0.745098, 0.745098, 0.745098, 1.0);
    pub const GREEN: Color = Color::from_rgba(0.0, 1.0, 0.0, 1.0);
    pub const GREEN_YELLOW: Color = Color::from_rgba(0.678431, 1.0, 0.184314, 1.0);
    pub const HONEYDEW: Color = Color::from_rgba(0.941176, 1.0, 0.941176, 1.0);
    pub const HOT_PINK: Color = Color::from_rgba(1.0, 0.411765, 0.705882, 1.0);
    pub const INDIAN_RED: Color = Color::from_rgba(0.803922, 0.360784, 0.360784, 1.0);
    pub const INDIGO: Color = Color::from_rgba(0.294118, 0.0, 0.509804, 1.0);
    pub const IVORY: Color = Color::from_rgba(1.0, 1.0, 0.941176, 1.0);
    pub const KHAKI: Color = Color::from_rgba(0.941176, 0.901961, 0.54902, 1.0);
    pub const LAVENDER: Color = Color::from_rgba(0.901961, 0.901961, 0.980392, 1.0);
    pub const LAVENDER_BLUSH: Color = Color::from_rgba(1.0, 0.941176, 0.960784, 1.0);
    pub const LAWN_GREEN: Color = Color::from_rgba(0.486275, 0.988235, 0.0, 1.0);
    pub const LEMON_CHIFFON: Color = Color::from_rgba(1.0, 0.980392, 0.803922, 1.0);
    pub const LIGHT_BLUE: Color = Color::from_rgba(0.678431, 0.847059, 0.901961, 1.0);
    pub const LIGHT_CORAL: Color = Color::from_rgba(0.941176, 0.501961, 0.501961, 1.0);
    pub const LIGHT_CYAN: Color = Color::from_rgba(0.878431, 1.0, 1.0, 1.0);
    pub const LIGHT_GOLDENROD: Color = Color::from_rgba(0.980392, 0.980392, 0.823529, 1.0);
    pub const LIGHT_GRAY: Color = Color::from_rgba(0.827451, 0.827451, 0.827451, 1.0);
    pub const LIGHT_GREEN: Color = Color::from_rgba(0.564706, 0.933333, 0.564706, 1.0);
    pub const LIGHT_PINK: Color = Color::from_rgba(1.0, 0.713726, 0.756863, 1.0);
    pub const LIGHT_SALMON: Color = Color::from_rgba(1.0, 0.627451, 0.478431, 1.0);
    pub const LIGHT_SEA_GREEN: Color = Color::from_rgba(0.12549, 0.698039, 0.666667, 1.0);
    pub const LIGHT_SKY_BLUE: Color = Color::from_rgba(0.529412, 0.807843, 0.980392, 1.0);
    pub const LIGHT_SLATE_GRAY: Color = Color::from_rgba(0.466667, 0.533333, 0.6, 1.0);
    pub const LIGHT_STEEL_BLUE: Color = Color::from_rgba(0.690196, 0.768627, 0.870588, 1.0);
    pub const LIGHT_YELLOW: Color = Color::from_rgba(1.0, 1.0, 0.878431, 1.0);
    pub const LIME: Color = Color::from_rgba(0.0, 1.0, 0.0, 1.0);
    pub const LIME_GREEN: Color = Color::from_rgba(0.196078, 0.803922, 0.196078, 1.0);
    pub const LINEN: Color = Color::from_rgba(0.980392, 0.941176, 0.901961, 1.0);
    pub const MAGENTA: Color = Color::from_rgba(1.0, 0.0, 1.0, 1.0);
    pub const MAROON: Color = Color::from_rgba(0.690196, 0.188235, 0.376471, 1.0);
    pub const MEDIUM_AQUAMARINE: Color = Color::from_rgba(0.4, 0.803922, 0.666667, 1.0);
    pub const MEDIUM_BLUE: Color = Color::from_rgba(0.0, 0.0, 0.803922, 1.0);
    pub const MEDIUM_ORCHID: Color = Color::from_rgba(0.729412, 0.333333, 0.827451, 1.0);
    pub const MEDIUM_PURPLE: Color = Color::from_rgba(0.576471, 0.439216, 0.858824, 1.0);
    pub const MEDIUM_SEA_GREEN: Color = Color::from_rgba(0.235294, 0.701961, 0.443137, 1.0);
    pub const MEDIUM_SLATE_BLUE: Color = Color::from_rgba(0.482353, 0.407843, 0.933333, 1.0);
    pub const MEDIUM_SPRING_GREEN: Color = Color::from_rgba(0.0, 0.980392, 0.603922, 1.0);
    pub const MEDIUM_TURQUOISE: Color = Color::from_rgba(0.282353, 0.819608, 0.8, 1.0);
    pub const MEDIUM_VIOLET_RED: Color = Color::from_rgba(0.780392, 0.0823529, 0.521569, 1.0);
    pub const MIDNIGHT_BLUE: Color = Color::from_rgba(0.0980392, 0.0980392, 0.439216, 1.0);
    pub const MINT_CREAM: Color = Color::from_rgba(0.960784, 1.0, 0.980392, 1.0);
    pub const MISTY_ROSE: Color = Color::from_rgba(1.0, 0.894118, 0.882353, 1.0);
    pub const MOCCASIN: Color = Color::from_rgba(1.0, 0.894118, 0.709804, 1.0);
    pub const NAVAJO_WHITE: Color = Color::from_rgba(1.0, 0.870588, 0.678431, 1.0);
    pub const NAVY_BLUE: Color = Color::from_rgba(0.0, 0.0, 0.501961, 1.0);
    pub const OLD_LACE: Color = Color::from_rgba(0.992157, 0.960784, 0.901961, 1.0);
    pub const OLIVE: Color = Color::from_rgba(0.501961, 0.501961, 0.0, 1.0);
    pub const OLIVE_DRAB: Color = Color::from_rgba(0.419608, 0.556863, 0.137255, 1.0);
    pub const ORANGE: Color = Color::from_rgba(1.0, 0.647059, 0.0, 1.0);
    pub const ORANGE_RED: Color = Color::from_rgba(1.0, 0.270588, 0.0, 1.0);
    pub const ORCHID: Color = Color::from_rgba(0.854902, 0.439216, 0.839216, 1.0);
    pub const PALE_GOLDENROD: Color = Color::from_rgba(0.933333, 0.909804, 0.666667, 1.0);
    pub const PALE_GREEN: Color = Color::from_rgba(0.596078, 0.984314, 0.596078, 1.0);
    pub const PALE_TURQUOISE: Color = Color::from_rgba(0.686275, 0.933333, 0.933333, 1.0);
    pub const PALE_VIOLET_RED: Color = Color::from_rgba(0.858824, 0.439216, 0.576471, 1.0);
    pub const PAPAYA_WHIP: Color = Color::from_rgba(1.0, 0.937255, 0.835294, 1.0);
    pub const PEACH_PUFF: Color = Color::from_rgba(1.0, 0.854902, 0.72549, 1.0);
    pub const PERU: Color = Color::from_rgba(0.803922, 0.521569, 0.247059, 1.0);
    pub const PINK: Color = Color::from_rgba(1.0, 0.752941, 0.796078, 1.0);
    pub const PLUM: Color = Color::from_rgba(0.866667, 0.627451, 0.866667, 1.0);
    pub const POWDER_BLUE: Color = Color::from_rgba(0.690196, 0.878431, 0.901961, 1.0);
    pub const PURPLE: Color = Color::from_rgba(0.627451, 0.12549, 0.941176, 1.0);
    pub const REBECCA_PURPLE: Color = Color::from_rgba(0.4, 0.2, 0.6, 1.0);
    pub const RED: Color = Color::from_rgba(1.0, 0.0, 0.0, 1.0);
    pub const ROSY_BROWN: Color = Color::from_rgba(0.737255, 0.560784, 0.560784, 1.0);
    pub const ROYAL_BLUE: Color = Color::from_rgba(0.254902, 0.411765, 0.882353, 1.0);
    pub const SADDLE_BROWN: Color = Color::from_rgba(0.545098, 0.270588, 0.0745098, 1.0);
    pub const SALMON: Color = Color::from_rgba(0.980392, 0.501961, 0.447059, 1.0);
    pub const SANDY_BROWN: Color = Color::from_rgba(0.956863, 0.643137, 0.376471, 1.0);
    pub const SEA_GREEN: Color = Color::from_rgba(0.180392, 0.545098, 0.341176, 1.0);
    pub const SEASHELL: Color = Color::from_rgba(1.0, 0.960784, 0.933333, 1.0);
    pub const SIENNA: Color = Color::from_rgba(0.627451, 0.321569, 0.176471, 1.0);
    pub const SILVER: Color = Color::from_rgba(0.752941, 0.752941, 0.752941, 1.0);
    pub const SKY_BLUE: Color = Color::from_rgba(0.529412, 0.807843, 0.921569, 1.0);
    pub const SLATE_BLUE: Color = Color::from_rgba(0.415686, 0.352941, 0.803922, 1.0);
    pub const SLATE_GRAY: Color = Color::from_rgba(0.439216, 0.501961, 0.564706, 1.0);
    pub const SNOW: Color = Color::from_rgba(1.0, 0.980392, 0.980392, 1.0);
    pub const SPRING_GREEN: Color = Color::from_rgba(0.0, 1.0, 0.498039, 1.0);
    pub const STEEL_BLUE: Color = Color::from_rgba(0.27451, 0.509804, 0.705882, 1.0);
    pub const TAN: Color = Color::from_rgba(0.823529, 0.705882, 0.54902, 1.0);
    pub const TEAL: Color = Color::from_rgba(0.0, 0.501961, 0.501961, 1.0);
    pub const THISTLE: Color = Color::from_rgba(0.847059, 0.74902, 0.847059, 1.0);
    pub const TOMATO: Color = Color::from_rgba(1.0, 0.388235, 0.278431, 1.0);
    pub const TURQUOISE: Color = Color::from_rgba(0.25098, 0.878431, 0.815686, 1.0);
    pub const VIOLET: Color = Color::from_rgba(0.933333, 0.509804, 0.933333, 1.0);
    pub const WEB_GRAY: Color = Color::from_rgba(0.501961, 0.501961, 0.501961, 1.0);
    pub const WEB_GREEN: Color = Color::from_rgba(0.0, 0.501961, 0.0, 1.0);
    pub const WEB_MAROON: Color = Color::from_rgba(0.501961, 0.0, 0.0, 1.0);
    pub const WEB_PURPLE: Color = Color::from_rgba(0.501961, 0.0, 0.501961, 1.0);
    pub const WHEAT: Color = Color::from_rgba(0.960784, 0.870588, 0.701961, 1.0);
    pub const WHITE_SMOKE: Color = Color::from_rgba(0.960784, 0.960784, 0.960784, 1.0);
    pub const YELLOW: Color = Color::from_rgba(1.0, 1.0, 0.0, 1.0);
    pub const YELLOW_GREEN: Color = Color::from_rgba(0.603922, 0.803922, 0.196078, 1.0);
}
