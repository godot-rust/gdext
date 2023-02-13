/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::inner::InnerColor;
use crate::builtin::GodotString;
use godot_ffi as sys;
use std::ops;
use sys::{ffi_methods, GodotFfi};

/// Color built-in type, in floating-point RGBA format.
///
/// Channel values are _typically_ in the range of 0 to 1, but this is not a requirement, and
/// values outside this range are explicitly allowed for e.g. High Dynamic Range (HDR).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct Color {
    /// The color's red component.
    pub r: f32,
    /// The color's green component.
    pub g: f32,
    /// The color's blue component.
    pub b: f32,
    /// The color's alpha component. A value of 0 means that the color is fully transparent. A
    /// value of 1 means that the color is fully opaque.
    pub a: f32,
}

impl Color {
    // TODO implement all the other color constants using code generation

    /// Transparent black.
    pub const TRANSPARENT_BLACK: Color = Self::from_rgba(0.0, 0.0, 0.0, 0.0);

    /// Transparent white.
    ///
    /// _Godot equivalent: `Color.TRANSPARENT`_
    pub const TRANSPARENT_WHITE: Color = Self::from_rgba(1.0, 1.0, 1.0, 0.0);

    /// Opaque black.
    pub const BLACK: Color = Self::from_rgba(0.0, 0.0, 0.0, 1.0);

    /// Opaque white.
    pub const WHITE: Color = Self::from_rgba(1.0, 1.0, 1.0, 1.0);

    /// Constructs a new `Color` with the given components.
    pub const fn from_rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Constructs a new `Color` with the given color components, and the alpha channel set to 1.
    pub const fn from_rgb(r: f32, g: f32, b: f32) -> Self {
        Self::from_rgba(r, g, b, 1.0)
    }

    /// Constructs a new `Color` with the given components as bytes. 0 is mapped to 0.0, 255 is
    /// mapped to 1.0.
    ///
    /// _Godot equivalent: the global `Color8` function_
    pub fn from_rgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::from_rgba(from_u8(r), from_u8(g), from_u8(b), from_u8(a))
    }

    /// Constructs a new `Color` with the given components as `u16` words. 0 is mapped to 0.0,
    /// 65535 (`0xffff`) is mapped to 1.0.
    pub fn from_rgba16(r: u16, g: u16, b: u16, a: u16) -> Self {
        Self::from_rgba(from_u16(r), from_u16(g), from_u16(b), from_u16(a))
    }

    /// Constructs a new `Color` from a 32-bits value with the given channel `order`.
    ///
    /// _Godot equivalent: `Color.hex`, if `ColorChannelOrder::Rgba` is used_
    pub fn from_u32_rgba(u: u32, order: ColorChannelOrder) -> Self {
        let [r, g, b, a] = order.unpack(u.to_be_bytes());
        Color::from_rgba8(r, g, b, a)
    }

    /// Constructs a new `Color` from a 64-bits value with the given channel `order`.
    ///
    /// _Godot equivalent: `Color.hex64`, if `ColorChannelOrder::Rgba` is used_
    pub fn from_u64_rgba(u: u64, order: ColorChannelOrder) -> Self {
        let [r, g, b, a] = order.unpack(to_be_words(u));
        Color::from_rgba16(r, g, b, a)
    }

    /// Constructs a `Color` from an HTML color code string. Valid values for the string are:
    ///
    /// - `#RRGGBBAA` and `RRGGBBAA` where each of `RR`, `GG`, `BB` and `AA` stands for two hex
    ///   digits (case insensitive).
    /// - `#RRGGBB` and `RRGGBB`. Equivalent to `#RRGGBBff`.
    /// - `#RGBA` and `RGBA` where each of `R`, `G`, `B` and `A` stands for a single hex digit.
    ///   Equivalent to `#RRGGBBAA`, i.e. each digit is repeated twice.
    /// - `#RGB` and `RGB`. Equivalent to `#RRGGBBff`.
    ///
    /// Returns `None` if the format is invalid.
    pub fn from_html<S: Into<GodotString>>(html: S) -> Option<Self> {
        let html = html.into();
        InnerColor::html_is_valid(html.clone()).then(|| InnerColor::html(html))
    }

    /// Constructs a `Color` from a string, which can be either:
    ///
    /// - An HTML color code as accepted by [`Color::from_html`].
    /// - The name of a built-in color constant, such as `BLUE` or `lawn-green`. Matching is case
    ///   insensitive and hyphens can be used interchangeably with underscores. See the [list of
    ///   color constants][color_constants] in the Godot API documentation, or the visual [cheat
    ///   sheet][cheat_sheet] for the full list.
    ///
    /// Returns `None` if the string is neither a valid HTML color code nor an existing color name.
    ///
    /// Most color constants have an alpha of 1; use [`Color::with_alpha`] to change it.
    ///
    /// [color_constants]: https://docs.godotengine.org/en/latest/classes/class_color.html#constants
    /// [cheat_sheet]: https://raw.githubusercontent.com/godotengine/godot-docs/master/img/color_constants.png
    pub fn from_string<S: Into<GodotString>>(string: S) -> Option<Self> {
        let color = InnerColor::from_string(
            string.into(),
            Self::from_rgba(f32::NAN, f32::NAN, f32::NAN, f32::NAN),
        );
        // Assumption: the implementation of `from_string` in the engine will never return any NaN
        // upon success.
        if color.r.is_nan() {
            None
        } else {
            Some(color)
        }
    }

    /// Constructs a `Color` from an [HSV profile](https://en.wikipedia.org/wiki/HSL_and_HSV). The
    /// hue (`h`), saturation (`s`), and value (`v`) are typically between 0.0 and 1.0. Alpha is
    /// set to 1; use [`Color::with_alpha`] to change it.
    pub fn from_hsv(h: f64, s: f64, v: f64) -> Self {
        InnerColor::from_hsv(h, s, v, 1.0)
    }

    /// Constructs a `Color` from an [OK HSL
    /// profile](https://bottosson.github.io/posts/colorpicker/). The hue (`h`), saturation (`s`),
    /// and lightness (`l`) are typically between 0.0 and 1.0. Alpha is set to 1; use
    /// [`Color::with_alpha`] to change it.
    pub fn from_ok_hsl(h: f64, s: f64, l: f64) -> Self {
        InnerColor::from_ok_hsl(h, s, l, 1.0)
    }

    /// Constructs a `Color` from an RGBE9995 format integer. This is a special OpenGL texture
    /// format where the three color components have 9 bits of precision and all three share a
    /// single 5-bit exponent.
    pub fn from_rgbe9995(rgbe: u32) -> Self {
        InnerColor::from_rgbe9995(rgbe as i64)
    }

    /// Returns a copy of this color with the given alpha value. Useful for chaining with
    /// constructors like [`Color::from_string`] and [`Color::from_hsv`].
    #[must_use]
    pub fn with_alpha(mut self, a: f32) -> Self {
        self.a = a;
        self
    }

    /// Returns the red channel value as a byte. If `self.r` is outside the range from 0 to 1, the
    /// returned byte is clamped.
    pub fn r8(self) -> u8 {
        to_u8(self.r)
    }

    /// Returns the green channel value as a byte. If `self.g` is outside the range from 0 to 1,
    /// the returned byte is clamped.
    pub fn g8(self) -> u8 {
        to_u8(self.g)
    }

    /// Returns the blue channel value as a byte. If `self.b` is outside the range from 0 to 1, the
    /// returned byte is clamped.
    pub fn b8(self) -> u8 {
        to_u8(self.b)
    }

    /// Returns the alpha channel value as a byte. If `self.a` is outside the range from 0 to 1,
    /// the returned byte is clamped.
    pub fn a8(self) -> u8 {
        to_u8(self.a)
    }

    /// Sets the red channel value as a byte, mapped to the range from 0 to 1.
    pub fn set_r8(&mut self, r: u8) {
        self.r = from_u8(r);
    }

    /// Sets the green channel value as a byte, mapped to the range from 0 to 1.
    pub fn set_g8(&mut self, g: u8) {
        self.g = from_u8(g);
    }

    /// Sets the blue channel value as a byte, mapped to the range from 0 to 1.
    pub fn set_b8(&mut self, b: u8) {
        self.b = from_u8(b);
    }

    /// Sets the alpha channel value as a byte, mapped to the range from 0 to 1.
    pub fn set_a8(&mut self, a: u8) {
        self.a = from_u8(a);
    }

    // TODO add getters and setters for h, s, v (needs generated property wrappers)

    /// Returns the light intensity of the color, as a value between 0.0 and 1.0 (inclusive). This
    /// is useful when determining whether a color is light or dark. Colors with a luminance
    /// smaller than 0.5 can be generally considered dark.
    ///
    /// Note: `luminance` relies on the color being in the linear color space to return an
    /// accurate relative luminance value. If the color is in the sRGB color space, use
    /// [`Color::srgb_to_linear`] to convert it to the linear color space first.
    pub fn luminance(self) -> f64 {
        self.as_inner().get_luminance()
    }

    /// Blends the given color on top of this color, taking its alpha into account.
    #[must_use]
    pub fn blend(self, over: Color) -> Self {
        self.as_inner().blend(over)
    }

    /// Returns the linear interpolation between `self`'s components and `to`'s components. The
    /// interpolation factor `weight` should be between 0.0 and 1.0 (inclusive).
    #[must_use]
    pub fn lerp(self, to: Color, weight: f64) -> Self {
        self.as_inner().lerp(to, weight)
    }

    /// Returns a new color with all components clamped between the components of `min` and `max`.
    #[must_use]
    pub fn clamp(self, min: Color, max: Color) -> Self {
        self.as_inner().clamp(min, max)
    }

    /// Creates a new color resulting by making this color darker by the specified amount (ratio
    /// from 0.0 to 1.0). See also [`lightened`].
    #[must_use]
    pub fn darkened(self, amount: f64) -> Self {
        self.as_inner().darkened(amount)
    }

    /// Creates a new color resulting by making this color lighter by the specified amount, which
    /// should be a ratio from 0.0 to 1.0. See also [`darken`].
    #[must_use]
    pub fn lightened(self, amount: f64) -> Self {
        self.as_inner().lightened(amount)
    }

    /// Returns the color with its `r`, `g`, and `b` components inverted:
    /// `Color::from_rgba(1 - r, 1 - g, 1 - b, a)`.
    #[must_use]
    pub fn inverted(self) -> Self {
        self.as_inner().inverted()
    }

    /// Returns the color converted to the [sRGB](https://en.wikipedia.org/wiki/SRGB) color space.
    /// This method assumes the original color is in the linear color space. See also
    /// [`Color::srgb_to_linear`] which performs the opposite operation.
    #[must_use]
    pub fn linear_to_srgb(self) -> Self {
        self.as_inner().linear_to_srgb()
    }

    /// Returns the color converted to the linear color space. This method assumes the original
    /// color is in the sRGB color space. See also [`Color::linear_to_srgb`] which performs the
    /// opposite operation.
    #[must_use]
    pub fn srgb_to_linear(self) -> Self {
        self.as_inner().srgb_to_linear()
    }

    /// Returns the HTML color code representation of this color, as 8 lowercase hex digits in the
    /// order `RRGGBBAA`, without the `#` prefix.
    pub fn to_html(self) -> GodotString {
        self.as_inner().to_html(true)
    }

    /// Returns the HTML color code representation of this color, as 6 lowercase hex digits in the
    /// order `RRGGBB`, without the `#` prefix. The alpha channel is ignored.
    pub fn to_html_without_alpha(self) -> GodotString {
        self.as_inner().to_html(false)
    }

    /// Returns true if `self` and `to` are approximately equal, within the tolerance used by
    /// the global `is_equal_approx` function in GDScript.
    pub fn is_equal_approx(self, to: Color) -> bool {
        self.as_inner().is_equal_approx(to)
    }

    /// Returns the color converted to a 32-bit integer (each component is 8 bits) with the given
    /// `order` of channels (from most to least significant byte).
    pub fn to_u32(self, order: ColorChannelOrder) -> u32 {
        u32::from_be_bytes(order.pack([to_u8(self.r), to_u8(self.g), to_u8(self.b), to_u8(self.a)]))
    }

    /// Returns the color converted to a 64-bit integer (each component is 16 bits) with the given
    /// `order` of channels (from most to least significant word).
    pub fn to_u64(self, order: ColorChannelOrder) -> u64 {
        from_be_words(order.pack([
            to_u16(self.r),
            to_u16(self.g),
            to_u16(self.b),
            to_u16(self.a),
        ]))
    }

    fn as_inner(&self) -> InnerColor {
        InnerColor::from_outer(self)
    }
}

impl GodotFfi for Color {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorChannelOrder {
    /// RGBA channel order. Godot's default.
    Rgba,
    /// ABGR channel order. Reverse of the default RGBA order.
    Abgr,
    /// ARGB channel order. More compatible with DirectX.
    Argb,
}

impl ColorChannelOrder {
    fn pack<T>(self, rgba: [T; 4]) -> [T; 4] {
        let [r, g, b, a] = rgba;
        match self {
            ColorChannelOrder::Rgba => [r, g, b, a],
            ColorChannelOrder::Abgr => [a, b, g, r],
            ColorChannelOrder::Argb => [a, r, g, b],
        }
    }

    fn unpack<T>(self, xyzw: [T; 4]) -> [T; 4] {
        let [x, y, z, w] = xyzw;
        match self {
            ColorChannelOrder::Rgba => [x, y, z, w],
            ColorChannelOrder::Abgr => [w, z, y, x],
            ColorChannelOrder::Argb => [y, z, w, x],
        }
    }
}

/// Constructs a default `Color` which is opaque black.
impl Default for Color {
    fn default() -> Self {
        Self::BLACK
    }
}

impl ops::Mul<Color> for Color {
    type Output = Color;
    fn mul(mut self, rhs: Color) -> Self::Output {
        self *= rhs;
        self
    }
}

impl ops::MulAssign<Color> for Color {
    fn mul_assign(&mut self, rhs: Color) {
        self.r *= rhs.r;
        self.g *= rhs.g;
        self.b *= rhs.b;
        self.a *= rhs.a;
    }
}

impl ops::Mul<Color> for f32 {
    type Output = Color;
    fn mul(self, mut rhs: Color) -> Self::Output {
        rhs *= self;
        rhs
    }
}

impl ops::Mul<f32> for Color {
    type Output = Color;
    fn mul(mut self, rhs: f32) -> Self::Output {
        self *= rhs;
        self
    }
}

impl ops::MulAssign<f32> for Color {
    fn mul_assign(&mut self, f: f32) {
        self.r *= f;
        self.g *= f;
        self.b *= f;
        self.a *= f;
    }
}

impl ops::Div<Color> for Color {
    type Output = Color;
    fn div(mut self, rhs: Color) -> Self::Output {
        self /= rhs;
        self
    }
}

impl ops::DivAssign<Color> for Color {
    fn div_assign(&mut self, rhs: Color) {
        self.r /= rhs.r;
        self.g /= rhs.g;
        self.b /= rhs.b;
        self.a /= rhs.a;
    }
}

impl ops::Add<Color> for Color {
    type Output = Color;
    fn add(mut self, rhs: Color) -> Self::Output {
        self += rhs;
        self
    }
}

impl ops::AddAssign<Color> for Color {
    fn add_assign(&mut self, rhs: Color) {
        self.r += rhs.r;
        self.g += rhs.g;
        self.b += rhs.b;
        self.a += rhs.a;
    }
}

impl ops::Sub<Color> for Color {
    type Output = Color;
    fn sub(mut self, rhs: Color) -> Self::Output {
        self -= rhs;
        self
    }
}

impl ops::SubAssign<Color> for Color {
    fn sub_assign(&mut self, rhs: Color) {
        self.r -= rhs.r;
        self.g -= rhs.g;
        self.b -= rhs.b;
        self.a -= rhs.a;
    }
}

impl ops::Neg for Color {
    type Output = Self;
    fn neg(self) -> Self {
        Self::from_rgba(-self.r, -self.g, -self.b, -self.a)
    }
}

/// Converts a single channel byte to a float in the range 0 to 1.
fn from_u8(byte: u8) -> f32 {
    byte as f32 / 255.0
}

/// Converts a single channel `u16` word to a float in the range 0 to 1.
fn from_u16(byte: u16) -> f32 {
    byte as f32 / 65535.0
}

/// Converts a float in the range 0 to 1 to a byte. Matches rounding behavior of the engine.
fn to_u8(v: f32) -> u8 {
    // core/math/color.h:
    // _FORCE_INLINE_ int32_t get_r8() const { return int32_t(CLAMP(Math::round(r * 255.0f), 0.0f, 255.0f)); }
    const MAX: f32 = 255.0;
    (v * MAX).round().clamp(0.0, MAX) as u8
}

/// Converts a float in the range 0 to 1 to a `u16` word. Matches rounding behavior of the engine.
fn to_u16(v: f32) -> u16 {
    // core/math/color.cpp:
    // uint64_t c = (uint16_t)Math::round(a * 65535.0f);
    // It does not clamp, but we do.
    const MAX: f32 = 65535.0;
    (v * MAX).round().clamp(0.0, MAX) as u16
}

/// Packs four `u16` words into a `u64` in big-endian order.
fn from_be_words(words: [u16; 4]) -> u64 {
    (words[0] as u64) << 48 | (words[1] as u64) << 32 | (words[2] as u64) << 16 | (words[3] as u64)
}

/// Unpacks a `u64` into four `u16` words in big-endian order.
fn to_be_words(mut u: u64) -> [u16; 4] {
    let w = (u & 0xffff) as u16;
    u >>= 16;
    let z = (u & 0xffff) as u16;
    u >>= 16;
    let y = (u & 0xffff) as u16;
    u >>= 16;
    let x = (u & 0xffff) as u16;
    [x, y, z, w]
}
