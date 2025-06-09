/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::{Color, ColorChannelOrder};

/// Godot's predefined colors.
///
/// This [visual cheat sheet](https://raw.githubusercontent.com/godotengine/godot-docs/master/img/color_constants.png) shows how the colors look.
///
/// For a systematic list of all predefined Godot colors, check out the [`ALL_GODOT_COLORS`][Self::ALL_GODOT_COLORS] constant.
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

    // All predefined values. Values sourced from testing Color::from_string() directly.

    pub const ALICE_BLUE: Color = rgba(0xfff0f8ff);
    pub const ANTIQUE_WHITE: Color = rgba(0xfffaebd7);
    pub const AQUA: Color = rgba(0xff00ffff);
    pub const AQUAMARINE: Color = rgba(0xff7fffd4);
    pub const AZURE: Color = rgba(0xfff0ffff);
    pub const BEIGE: Color = rgba(0xfff5f5dc);
    pub const BISQUE: Color = rgba(0xffffe4c4);
    /// Black color. This is the [default](Color::default) value.
    pub const BLACK: Color = rgba(0xff000000);
    pub const BLANCHED_ALMOND: Color = rgba(0xffffebcd);
    pub const BLUE: Color = rgba(0xff0000ff);
    pub const BLUE_VIOLET: Color = rgba(0xff8a2be2);
    pub const BROWN: Color = rgba(0xffa52a2a);
    pub const BURLYWOOD: Color = rgba(0xffdeb887);
    pub const CADET_BLUE: Color = rgba(0xff5f9ea0);
    pub const CHARTREUSE: Color = rgba(0xff7fff00);
    pub const CHOCOLATE: Color = rgba(0xffd2691e);
    pub const CORAL: Color = rgba(0xffff7f50);
    pub const CORNFLOWER_BLUE: Color = rgba(0xff6495ed);
    pub const CORNSILK: Color = rgba(0xfffff8dc);
    pub const CRIMSON: Color = rgba(0xffdc143c);
    pub const CYAN: Color = rgba(0xff00ffff);
    pub const DARK_BLUE: Color = rgba(0xff00008b);
    pub const DARK_CYAN: Color = rgba(0xff008b8b);
    pub const DARK_GOLDENROD: Color = rgba(0xffb8860b);
    pub const DARK_GRAY: Color = rgba(0xffa9a9a9);
    pub const DARK_GREEN: Color = rgba(0xff006400);
    pub const DARK_KHAKI: Color = rgba(0xffbdb76b);
    pub const DARK_MAGENTA: Color = rgba(0xff8b008b);
    pub const DARK_OLIVE_GREEN: Color = rgba(0xff556b2f);
    pub const DARK_ORANGE: Color = rgba(0xffff8c00);
    pub const DARK_ORCHID: Color = rgba(0xff9932cc);
    pub const DARK_RED: Color = rgba(0xff8b0000);
    pub const DARK_SALMON: Color = rgba(0xffe9967a);
    pub const DARK_SEA_GREEN: Color = rgba(0xff8fbc8f);
    pub const DARK_SLATE_BLUE: Color = rgba(0xff483d8b);
    pub const DARK_SLATE_GRAY: Color = rgba(0xff2f4f4f);
    pub const DARK_TURQUOISE: Color = rgba(0xff00ced1);
    pub const DARK_VIOLET: Color = rgba(0xff9400d3);
    pub const DEEP_PINK: Color = rgba(0xffff1493);
    pub const DEEP_SKY_BLUE: Color = rgba(0xff00bfff);
    pub const DIM_GRAY: Color = rgba(0xff696969);
    pub const DODGER_BLUE: Color = rgba(0xff1e90ff);
    pub const FIREBRICK: Color = rgba(0xffb22222);
    pub const FLORAL_WHITE: Color = rgba(0xfffffaf0);
    pub const FOREST_GREEN: Color = rgba(0xff228b22);
    pub const FUCHSIA: Color = rgba(0xffff00ff);
    pub const GAINSBORO: Color = rgba(0xffdcdcdc);
    pub const GHOST_WHITE: Color = rgba(0xfff8f8ff);
    pub const GOLD: Color = rgba(0xffffd700);
    pub const GOLDENROD: Color = rgba(0xffdaa520);
    pub const GRAY: Color = rgba(0xffbebebe);
    pub const GREEN: Color = rgba(0xff00ff00);
    pub const GREEN_YELLOW: Color = rgba(0xffadff2f);
    pub const HONEYDEW: Color = rgba(0xfff0fff0);
    pub const HOT_PINK: Color = rgba(0xffff69b4);
    pub const INDIAN_RED: Color = rgba(0xffcd5c5c);
    pub const INDIGO: Color = rgba(0xff4b0082);
    pub const IVORY: Color = rgba(0xfffffff0);
    pub const KHAKI: Color = rgba(0xfff0e68c);
    pub const LAVENDER: Color = rgba(0xffe6e6fa);
    pub const LAVENDER_BLUSH: Color = rgba(0xfffff0f5);
    pub const LAWN_GREEN: Color = rgba(0xff7cfc00);
    pub const LEMON_CHIFFON: Color = rgba(0xfffffacd);
    pub const LIGHT_BLUE: Color = rgba(0xffadd8e6);
    pub const LIGHT_CORAL: Color = rgba(0xfff08080);
    pub const LIGHT_CYAN: Color = rgba(0xffe0ffff);
    pub const LIGHT_GOLDENROD: Color = rgba(0xfffafad2);
    pub const LIGHT_GRAY: Color = rgba(0xffd3d3d3);
    pub const LIGHT_GREEN: Color = rgba(0xff90ee90);
    pub const LIGHT_PINK: Color = rgba(0xffffb6c1);
    pub const LIGHT_SALMON: Color = rgba(0xffffa07a);
    pub const LIGHT_SEA_GREEN: Color = rgba(0xff20b2aa);
    pub const LIGHT_SKY_BLUE: Color = rgba(0xff87cefa);
    pub const LIGHT_SLATE_GRAY: Color = rgba(0xff778899);
    pub const LIGHT_STEEL_BLUE: Color = rgba(0xffb0c4de);
    pub const LIGHT_YELLOW: Color = rgba(0xffffffe0);
    pub const LIME: Color = rgba(0xff00ff00);
    pub const LIME_GREEN: Color = rgba(0xff32cd32);
    pub const LINEN: Color = rgba(0xfffaf0e6);
    pub const MAGENTA: Color = rgba(0xffff00ff);
    pub const MAROON: Color = rgba(0xffb03060);
    pub const MEDIUM_AQUAMARINE: Color = rgba(0xff66cdaa);
    pub const MEDIUM_BLUE: Color = rgba(0xff0000cd);
    pub const MEDIUM_ORCHID: Color = rgba(0xffba55d3);
    pub const MEDIUM_PURPLE: Color = rgba(0xff9370db);
    pub const MEDIUM_SEA_GREEN: Color = rgba(0xff3cb371);
    pub const MEDIUM_SLATE_BLUE: Color = rgba(0xff7b68ee);
    pub const MEDIUM_SPRING_GREEN: Color = rgba(0xff00fa9a);
    pub const MEDIUM_TURQUOISE: Color = rgba(0xff48d1cc);
    pub const MEDIUM_VIOLET_RED: Color = rgba(0xffc71585);
    pub const MIDNIGHT_BLUE: Color = rgba(0xff191970);
    pub const MINT_CREAM: Color = rgba(0xfff5fffa);
    pub const MISTY_ROSE: Color = rgba(0xffffe4e1);
    pub const MOCCASIN: Color = rgba(0xffffe4b5);
    pub const NAVAJO_WHITE: Color = rgba(0xffffdead);
    pub const NAVY_BLUE: Color = rgba(0xff000080);
    pub const OLD_LACE: Color = rgba(0xfffdf5e6);
    pub const OLIVE: Color = rgba(0xff808000);
    pub const OLIVE_DRAB: Color = rgba(0xff6b8e23);
    pub const ORANGE: Color = rgba(0xffffa500);
    pub const ORANGE_RED: Color = rgba(0xffff4500);
    pub const ORCHID: Color = rgba(0xffda70d6);
    pub const PALE_GOLDENROD: Color = rgba(0xffeee8aa);
    pub const PALE_GREEN: Color = rgba(0xff98fb98);
    pub const PALE_TURQUOISE: Color = rgba(0xffafeeee);
    pub const PALE_VIOLET_RED: Color = rgba(0xffdb7093);
    pub const PAPAYA_WHIP: Color = rgba(0xffffefd5);
    pub const PEACH_PUFF: Color = rgba(0xffffdab9);
    pub const PERU: Color = rgba(0xffcd853f);
    pub const PINK: Color = rgba(0xffffc0cb);
    pub const PLUM: Color = rgba(0xffdda0dd);
    pub const POWDER_BLUE: Color = rgba(0xffb0e0e6);
    pub const PURPLE: Color = rgba(0xffa020f0);
    pub const REBECCA_PURPLE: Color = rgba(0xff663399);
    pub const RED: Color = rgba(0xffff0000);
    pub const ROSY_BROWN: Color = rgba(0xffbc8f8f);
    pub const ROYAL_BLUE: Color = rgba(0xff4169e1);
    pub const SADDLE_BROWN: Color = rgba(0xff8b4513);
    pub const SALMON: Color = rgba(0xfffa8072);
    pub const SANDY_BROWN: Color = rgba(0xfff4a460);
    pub const SEA_GREEN: Color = rgba(0xff2e8b57);
    pub const SEASHELL: Color = rgba(0xfffff5ee);
    pub const SIENNA: Color = rgba(0xffa0522d);
    pub const SILVER: Color = rgba(0xffc0c0c0);
    pub const SKY_BLUE: Color = rgba(0xff87ceeb);
    pub const SLATE_BLUE: Color = rgba(0xff6a5acd);
    pub const SLATE_GRAY: Color = rgba(0xff708090);
    pub const SNOW: Color = rgba(0xfffffafa);
    pub const SPRING_GREEN: Color = rgba(0xff00ff7f);
    pub const STEEL_BLUE: Color = rgba(0xff4682b4);
    pub const TAN: Color = rgba(0xffd2b48c);
    pub const TEAL: Color = rgba(0xff008080);
    pub const THISTLE: Color = rgba(0xffd8bfd8);
    pub const TOMATO: Color = rgba(0xffff6347);
    pub const TURQUOISE: Color = rgba(0xff40e0d0);
    pub const VIOLET: Color = rgba(0xffee82ee);
    pub const WEB_GRAY: Color = rgba(0xff808080);
    pub const WEB_GREEN: Color = rgba(0xff008000);
    pub const WEB_MAROON: Color = rgba(0xff800000);
    pub const WEB_PURPLE: Color = rgba(0xff800080);
    pub const WHEAT: Color = rgba(0xfff5deb3);
    pub const WHITE: Color = rgba(0xffffffff);
    pub const WHITE_SMOKE: Color = rgba(0xfff5f5f5);
    pub const YELLOW: Color = rgba(0xffffff00);
    pub const YELLOW_GREEN: Color = rgba(0xff9acd32);

    /// All colors that Godot itself defines on the `Color` builtin type, in alphabetic order.
    ///
    /// Contains tuples where the first element is the name of the color constant **as available in Godot**, and the second element is the
    /// corresponding [`Color`]. For each tuple `(name, color)`, the property `Color::from_string(name) == color` holds.
    ///
    /// Excludes Rust-specific additions like `TRANSPARENT_BLACK` and `TRANSPARENT_WHITE` (however includes `TRANSPARENT`).
    ///
    /// This list may change over time.
    pub const ALL_GODOT_COLORS: &'static [(&'static str, Color)] = &[
        ("ALICE_BLUE", Self::ALICE_BLUE),
        ("ANTIQUE_WHITE", Self::ANTIQUE_WHITE),
        ("AQUA", Self::AQUA),
        ("AQUAMARINE", Self::AQUAMARINE),
        ("AZURE", Self::AZURE),
        ("BEIGE", Self::BEIGE),
        ("BISQUE", Self::BISQUE),
        ("BLACK", Self::BLACK),
        ("BLANCHED_ALMOND", Self::BLANCHED_ALMOND),
        ("BLUE", Self::BLUE),
        ("BLUE_VIOLET", Self::BLUE_VIOLET),
        ("BROWN", Self::BROWN),
        ("BURLYWOOD", Self::BURLYWOOD),
        ("CADET_BLUE", Self::CADET_BLUE),
        ("CHARTREUSE", Self::CHARTREUSE),
        ("CHOCOLATE", Self::CHOCOLATE),
        ("CORAL", Self::CORAL),
        ("CORNFLOWER_BLUE", Self::CORNFLOWER_BLUE),
        ("CORNSILK", Self::CORNSILK),
        ("CRIMSON", Self::CRIMSON),
        ("CYAN", Self::CYAN),
        ("DARK_BLUE", Self::DARK_BLUE),
        ("DARK_CYAN", Self::DARK_CYAN),
        ("DARK_GOLDENROD", Self::DARK_GOLDENROD),
        ("DARK_GRAY", Self::DARK_GRAY),
        ("DARK_GREEN", Self::DARK_GREEN),
        ("DARK_KHAKI", Self::DARK_KHAKI),
        ("DARK_MAGENTA", Self::DARK_MAGENTA),
        ("DARK_OLIVE_GREEN", Self::DARK_OLIVE_GREEN),
        ("DARK_ORANGE", Self::DARK_ORANGE),
        ("DARK_ORCHID", Self::DARK_ORCHID),
        ("DARK_RED", Self::DARK_RED),
        ("DARK_SALMON", Self::DARK_SALMON),
        ("DARK_SEA_GREEN", Self::DARK_SEA_GREEN),
        ("DARK_SLATE_BLUE", Self::DARK_SLATE_BLUE),
        ("DARK_SLATE_GRAY", Self::DARK_SLATE_GRAY),
        ("DARK_TURQUOISE", Self::DARK_TURQUOISE),
        ("DARK_VIOLET", Self::DARK_VIOLET),
        ("DEEP_PINK", Self::DEEP_PINK),
        ("DEEP_SKY_BLUE", Self::DEEP_SKY_BLUE),
        ("DIM_GRAY", Self::DIM_GRAY),
        ("DODGER_BLUE", Self::DODGER_BLUE),
        ("FIREBRICK", Self::FIREBRICK),
        ("FLORAL_WHITE", Self::FLORAL_WHITE),
        ("FOREST_GREEN", Self::FOREST_GREEN),
        ("FUCHSIA", Self::FUCHSIA),
        ("GAINSBORO", Self::GAINSBORO),
        ("GHOST_WHITE", Self::GHOST_WHITE),
        ("GOLD", Self::GOLD),
        ("GOLDENROD", Self::GOLDENROD),
        ("GRAY", Self::GRAY),
        ("GREEN", Self::GREEN),
        ("GREEN_YELLOW", Self::GREEN_YELLOW),
        ("HONEYDEW", Self::HONEYDEW),
        ("HOT_PINK", Self::HOT_PINK),
        ("INDIAN_RED", Self::INDIAN_RED),
        ("INDIGO", Self::INDIGO),
        ("IVORY", Self::IVORY),
        ("KHAKI", Self::KHAKI),
        ("LAVENDER", Self::LAVENDER),
        ("LAVENDER_BLUSH", Self::LAVENDER_BLUSH),
        ("LAWN_GREEN", Self::LAWN_GREEN),
        ("LEMON_CHIFFON", Self::LEMON_CHIFFON),
        ("LIGHT_BLUE", Self::LIGHT_BLUE),
        ("LIGHT_CORAL", Self::LIGHT_CORAL),
        ("LIGHT_CYAN", Self::LIGHT_CYAN),
        ("LIGHT_GOLDENROD", Self::LIGHT_GOLDENROD),
        ("LIGHT_GRAY", Self::LIGHT_GRAY),
        ("LIGHT_GREEN", Self::LIGHT_GREEN),
        ("LIGHT_PINK", Self::LIGHT_PINK),
        ("LIGHT_SALMON", Self::LIGHT_SALMON),
        ("LIGHT_SEA_GREEN", Self::LIGHT_SEA_GREEN),
        ("LIGHT_SKY_BLUE", Self::LIGHT_SKY_BLUE),
        ("LIGHT_SLATE_GRAY", Self::LIGHT_SLATE_GRAY),
        ("LIGHT_STEEL_BLUE", Self::LIGHT_STEEL_BLUE),
        ("LIGHT_YELLOW", Self::LIGHT_YELLOW),
        ("LIME", Self::LIME),
        ("LIME_GREEN", Self::LIME_GREEN),
        ("LINEN", Self::LINEN),
        ("MAGENTA", Self::MAGENTA),
        ("MAROON", Self::MAROON),
        ("MEDIUM_AQUAMARINE", Self::MEDIUM_AQUAMARINE),
        ("MEDIUM_BLUE", Self::MEDIUM_BLUE),
        ("MEDIUM_ORCHID", Self::MEDIUM_ORCHID),
        ("MEDIUM_PURPLE", Self::MEDIUM_PURPLE),
        ("MEDIUM_SEA_GREEN", Self::MEDIUM_SEA_GREEN),
        ("MEDIUM_SLATE_BLUE", Self::MEDIUM_SLATE_BLUE),
        ("MEDIUM_SPRING_GREEN", Self::MEDIUM_SPRING_GREEN),
        ("MEDIUM_TURQUOISE", Self::MEDIUM_TURQUOISE),
        ("MEDIUM_VIOLET_RED", Self::MEDIUM_VIOLET_RED),
        ("MIDNIGHT_BLUE", Self::MIDNIGHT_BLUE),
        ("MINT_CREAM", Self::MINT_CREAM),
        ("MISTY_ROSE", Self::MISTY_ROSE),
        ("MOCCASIN", Self::MOCCASIN),
        ("NAVAJO_WHITE", Self::NAVAJO_WHITE),
        ("NAVY_BLUE", Self::NAVY_BLUE),
        ("OLD_LACE", Self::OLD_LACE),
        ("OLIVE", Self::OLIVE),
        ("OLIVE_DRAB", Self::OLIVE_DRAB),
        ("ORANGE", Self::ORANGE),
        ("ORANGE_RED", Self::ORANGE_RED),
        ("ORCHID", Self::ORCHID),
        ("PALE_GOLDENROD", Self::PALE_GOLDENROD),
        ("PALE_GREEN", Self::PALE_GREEN),
        ("PALE_TURQUOISE", Self::PALE_TURQUOISE),
        ("PALE_VIOLET_RED", Self::PALE_VIOLET_RED),
        ("PAPAYA_WHIP", Self::PAPAYA_WHIP),
        ("PEACH_PUFF", Self::PEACH_PUFF),
        ("PERU", Self::PERU),
        ("PINK", Self::PINK),
        ("PLUM", Self::PLUM),
        ("POWDER_BLUE", Self::POWDER_BLUE),
        ("PURPLE", Self::PURPLE),
        ("REBECCA_PURPLE", Self::REBECCA_PURPLE),
        ("RED", Self::RED),
        ("ROSY_BROWN", Self::ROSY_BROWN),
        ("ROYAL_BLUE", Self::ROYAL_BLUE),
        ("SADDLE_BROWN", Self::SADDLE_BROWN),
        ("SALMON", Self::SALMON),
        ("SANDY_BROWN", Self::SANDY_BROWN),
        ("SEA_GREEN", Self::SEA_GREEN),
        ("SEASHELL", Self::SEASHELL),
        ("SIENNA", Self::SIENNA),
        ("SILVER", Self::SILVER),
        ("SKY_BLUE", Self::SKY_BLUE),
        ("SLATE_BLUE", Self::SLATE_BLUE),
        ("SLATE_GRAY", Self::SLATE_GRAY),
        ("SNOW", Self::SNOW),
        ("SPRING_GREEN", Self::SPRING_GREEN),
        ("STEEL_BLUE", Self::STEEL_BLUE),
        ("TAN", Self::TAN),
        ("TEAL", Self::TEAL),
        ("THISTLE", Self::THISTLE),
        ("TOMATO", Self::TOMATO),
        ("TRANSPARENT", Self::TRANSPARENT_WHITE),
        ("TURQUOISE", Self::TURQUOISE),
        ("VIOLET", Self::VIOLET),
        ("WEB_GRAY", Self::WEB_GRAY),
        ("WEB_GREEN", Self::WEB_GREEN),
        ("WEB_MAROON", Self::WEB_MAROON),
        ("WEB_PURPLE", Self::WEB_PURPLE),
        ("WHEAT", Self::WHEAT),
        ("WHITE", Self::WHITE),
        ("WHITE_SMOKE", Self::WHITE_SMOKE),
        ("YELLOW", Self::YELLOW),
        ("YELLOW_GREEN", Self::YELLOW_GREEN),
    ];
}

const fn rgba(value: u32) -> Color {
    Color::from_u32_rgba(value, ColorChannelOrder::ARGB)
}
