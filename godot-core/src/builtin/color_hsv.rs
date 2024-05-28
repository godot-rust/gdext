/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::math::{ApproxEq, FloatExt};
use super::Color;

/// HSVA floating-number Color representation.
///
/// Godot's [`Color`] built-in type supports mainly RGBA floating point-based notation. `ColorHsv` supports manipulating its HSVA
/// representation by introducing conversion methods between itself and `Color`.
///
/// `ColorHsv` *is not* a [`GodotType`](crate::meta::GodotType). To use it in properties expecting `Color`, you need to convert
/// it back to this type.
///
/// A `Color` created by [`ColorHsv::to_rgb()`] is equal to one created by [`Color::from_hsv(h, s, v)`](Color::from_hsv), but the conversion
/// time is approximately 4 times faster - partly because it avoids calls between Godot and Rust during conversion.
///
/// ## Conversions
///
/// Both conversions (`Color` to `ColorHsv` and `ColorHsv` to `Color`) will panic if RGBA or HSVA values are not within range `0.0..=1.0`.
/// To ensure the values are in valid range, methods [`Color::normalized`] and [`ColorHsv::normalized_clamped_h`]
/// or [`ColorHsv::normalized_wrapped_h`] can be used.
///
/// ```
/// use godot::builtin::{Color, ColorHsv};
///
/// let rgb = Color::from_rgb(1.15, 0.0, 0.0);
/// let hsv = ColorHsv::from_hsv(1.15, 0.0, 0.0);
/// ```
///
/// Such colors can't be converted - below calls will panic, because at least one of the color values are not within `0.0..=1.0` range.
///
/// ```should_panic
/// # use godot::builtin::{Color, ColorHsv};
/// # let rgb = Color::from_rgb(1.15, 0.0, 0.0);
/// let hsv_from_rgb = rgb.to_hsv();
/// ```
/// ```should_panic
/// # use godot::builtin::{Color, ColorHsv};
/// # let hsv = ColorHsv::from_hsv(1.15, 0.0, 0.0);
/// let rgb_from_hsv = hsv.to_rgb();
/// ```
///
/// After normalization all values are within `0.0..=1.0` range, so the conversions are safe and won't panic.
///
/// ```
/// # use godot::builtin::{Color, ColorHsv};
/// #
/// # let rgb = Color::from_rgb(1.15, 0.0, 0.0);
/// # let hsv = ColorHsv::from_hsv(1.15, 0.0, 0.0);
/// let hsv_from_rgb = rgb.normalized().to_hsv();
/// let rgb_from_hsv = hsv.normalized_wrapped_h().to_rgb();
/// ```
///
/// ## Precision warning
/// Conversions between `f32`-based RGB and HSV representations are not completely lossless. Try to avoid repeatable
/// `Color` -> `ColorHsv` -> `Color` roundtrips. One way to minimalize possible distortions is to keep `ColorHsv` on the Rust side, apply
/// all changes to this struct and convert it to `Color` before moving to the Godot side, instead of fetching `Color` from Godot side before
/// every mutation, though changes should be minimal if color values are mutated either only on `Color` or `ColorHsv` side.
///
/// ## Examples
///
/// ```
/// use godot::builtin::{Color, ColorHsv};
/// use godot::builtin::math::assert_eq_approx;
///
/// // ColorHsv can be constructed from only Hue, Saturation and Value.  
/// let mut c_hsv = ColorHsv::from_hsv(0.74, 0.69, 0.18);
///
/// // Or with Alpha value also specified. If not specified, it is set at 1.0.
/// let mut c_hsv2 = ColorHsv::from_hsva(0.74, 0.69, 0.18, 1.0);
///
/// assert_eq!(c_hsv, c_hsv2);
///
/// // Two way conversion: Color -> ColorHsv -> Color is not entirely lossless. Such repeatable
/// // conversions should be avoided, as the data loss could build up to significant values if values
/// // are mutated both on `Color` and `ColorHsv`.
/// let color1 = Color::from_rgb(0.74, 0.69, 0.18);
/// let color2 = color1.to_hsv().to_rgb();
///
/// assert_ne!(color1, color2);
/// assert_eq_approx!(color1.r, color2.r);
/// assert_eq_approx!(color1.g, color2.g);
/// assert_eq_approx!(color1.b, color2.b);
/// ```
///  
/// ## Reference
/// - Smith, Alvy Ray. "Color gamut transform pairs." ACM Siggraph Computer Graphics 12.3 (1978): 12-19.
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ColorHsv {
    pub h: f32,
    pub s: f32,
    pub v: f32,
    pub a: f32,
}

impl ApproxEq for ColorHsv {
    /// Hue values are wrapped before approximate comparison.
    fn approx_eq(&self, other: &Self) -> bool {
        (wrap_hue(self.h - other.h).is_zero_approx())
            && (self.s - other.s).abs().is_zero_approx()
            && (self.v - other.v).abs().is_zero_approx()
            && (self.a - other.a).abs().is_zero_approx()
    }
}

impl ColorHsv {
    /// Construct from Hue, Saturation and Value.
    ///
    /// Alpha will be set at `1.` by default. To construct with custom Alpha value, use [`ColorHsv::from_hsva`] constructor.
    pub const fn from_hsv(h: f32, s: f32, v: f32) -> Self {
        ColorHsv { h, s, v, a: 1.0 }
    }

    /// Construct from Hue, Saturation, Value and Alpha.
    ///
    /// To construct with Alpha set as default `1.`, use [`ColorHsv::from_hsv`] constructor.
    pub const fn from_hsva(h: f32, s: f32, v: f32, a: f32) -> Self {
        ColorHsv { h, s, v, a }
    }

    /// Transforms the `ColorHsv` into one with values clamped to the range valid for transformation into [`Color`].
    ///
    /// To normalize with **Hue** value wrapped, not clamped (for continuity around the hue wheel), use [`ColorHsv::normalized_wrapped_h`].
    ///
    /// ## Example
    ///
    /// ```
    /// use godot::builtin::ColorHsv;
    /// use godot::builtin::math::assert_eq_approx;
    ///
    /// let hsv_c = ColorHsv::from_hsv(1.35, -0.60, 1.15);
    /// let normalized = hsv_c.normalized_clamped_h();
    /// assert_eq_approx!(normalized, ColorHsv::from_hsv(1.0, 0.0, 1.0));
    /// ```
    #[must_use]
    pub fn normalized_clamped_h(self) -> Self {
        ColorHsv {
            h: self.h.clamp(0.0, 1.0),
            s: self.s.clamp(0.0, 1.0),
            v: self.v.clamp(0.0, 1.0),
            a: self.a.clamp(0.0, 1.0),
        }
    }

    /// Transforms the `ColorHsv` into one with **Hue** value wrapped and SVA clamped to the range valid for transformation into [`Color`].
    ///
    /// To normalize with **Hue** value clamped in the same way as SVA, use [`ColorHsv::normalized_clamped_h`].
    ///
    /// ## Example
    ///
    /// ```
    /// use godot::builtin::ColorHsv;
    /// use godot::builtin::math::assert_eq_approx;
    ///
    /// let hsv_c = ColorHsv::from_hsv(1.35, -0.60, 1.15);
    /// let normalized = hsv_c.normalized_wrapped_h();
    /// assert_eq_approx!(normalized, ColorHsv::from_hsv(0.35, 0.0, 1.0));
    /// ```
    #[must_use]
    pub fn normalized_wrapped_h(self) -> Self {
        ColorHsv {
            h: wrap_hue(self.h),
            s: self.s.clamp(0.0, 1.0),
            v: self.v.clamp(0.0, 1.0),
            a: self.a.clamp(0.0, 1.0),
        }
    }

    /// ⚠️ Convert `ColorHsv` into [`Color`].
    ///
    /// # Panics
    ///
    /// Method will panic if the HSVA values are outside of the valid range `0.0..=1.0`. You can use [`ColorHsv::normalized_clamped_h`] or
    /// [`ColorHsv::normalized_wrapped_h`] to ensure they are in range, or use [`ColorHsv::try_to_rgb`] implementation.
    pub fn to_rgb(self) -> Color {
        self.try_to_rgb().unwrap_or_else(|e| panic!("{e}"))
    }

    /// Fallible `ColorHsv` conversion into [`Color`]. See also: [`ColorHsv::to_rgb`].
    pub fn try_to_rgb(self) -> Result<Color, String> {
        if !self.is_normalized() {
            return Err(format!("HSVA values need to be in range `0.0..=1.0` before conversion, but were {self:?}. See: `ColorHsv::normalized_*()` methods."));
        }

        let (r, g, b, a) = hsva_to_rgba(self.h, self.s, self.v, self.a);
        Ok(Color { r, g, b, a })
    }

    fn is_normalized(&self) -> bool {
        self.h >= 0.0
            && self.h <= 1.0
            && self.s >= 0.0
            && self.s <= 1.0
            && self.v >= 0.0
            && self.v <= 1.0
            && self.a >= 0.0
            && self.a <= 1.0
    }
}

impl Default for ColorHsv {
    fn default() -> Self {
        Self {
            h: 0.0,
            s: 0.0,
            v: 0.0,
            a: 1.0,
        }
    }
}

pub(crate) fn rgba_to_hsva(r: f32, g: f32, b: f32, a: f32) -> (f32, f32, f32, f32) {
    let min = r.min(g).min(b);
    let max = r.max(g).max(b);

    let mut h: f32;
    let s: f32;
    let v = max;

    let delta = max - min;

    if delta.is_zero_approx() {
        s = 0.0;
        h = 0.0;

        return (h, s, v, a);
    }

    s = delta / max;

    if max == r {
        h = (g - b) / delta;
        if h < 0.0 {
            h += 6.0;
        }
    } else if max == g {
        h = 2.0 + (b - r) / delta;
    } else {
        h = 4.0 + (r - g) / delta;
    }

    h /= 6.0;

    (h, s, v, a)
}

fn hsva_to_rgba(h: f32, s: f32, v: f32, a: f32) -> (f32, f32, f32, f32) {
    if s.is_zero_approx() {
        return (v, v, v, a);
    }

    let h = h * 6.;
    let i = h.floor();
    let f = h - i;

    let m = v * (1.0 - s);
    let n = v * (1.0 - (s * f));
    let k = v * (1.0 - (s * (1.0 - f)));

    let (r, g, b) = match i as u8 {
        0 => (v, k, m),
        1 => (n, v, m),
        2 => (m, v, k),
        3 => (m, n, v),
        4 => (k, m, v),
        5 => (v, m, n),
        6 => (v, k, m),
        // Below shouldn't ever happen, because Hue value is checked to be in range of `0.0..=1.0`, so the maximum floored value of Hue * 6
        // will always be 6.
        _ => unreachable!(),
    };

    (r, g, b, a)
}

fn wrap_hue(hue: f32) -> f32 {
    // When running benchmarks, the `(0.0..1.0).contains(&hue)` were 2x slower than manual implementation.
    #[allow(clippy::manual_range_contains)]
    if hue >= 0.0 && hue < 1.0 {
        return hue;
    }
    if hue < 0.0 {
        return 1. + (hue % 1.0);
    }
    hue % 1.
}
