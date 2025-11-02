/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#[macro_export]
macro_rules! impl_shared_string_api {
    (
        builtin: $Builtin:ty,
        find_builder: $FindBuilder:ident,
        split_builder: $SplitBuilder:ident,
    ) => {
        // --------------------------------------------------------------------------------------------------------------------------------------
        // Extending the builtin itself

        /// Manually-declared, shared methods between `GString` and `StringName`.
        impl $Builtin {
            /// Returns the Unicode code point ("character") at position `index`.
            ///
            /// # Panics (safeguards-balanced)
            /// If `index` is out of bounds. In disengaged level, `0` is returned instead.
            // Unicode conversion panic is not documented because we rely on Godot strings having valid Unicode.
            // TODO implement Index/IndexMut (for GString; StringName may have varying reprs).
            pub fn unicode_at(&self, index: usize) -> char {
                sys::balanced_assert!(index < self.len(), "unicode_at: index {} out of bounds (len {})", index, self.len());

                let char_i64 = self.as_inner().unicode_at(index as i64);

                u32::try_from(char_i64).ok()
                    .and_then(|char_u32| char::from_u32(char_u32))
                    .unwrap_or_else(|| {
                        panic!("cannot map Unicode code point (value {char_i64}) to char (at position {index})")
                    })
            }

            /// Find first occurrence of `what` and return index, or `None` if not found.
            ///
            /// Check [`find_ex()`](Self::find_ex) for all custom options.
            pub fn find(&self, what: impl AsArg<GString>) -> Option<usize> {
                self.find_ex(what).done()
            }

            /// Returns a builder for finding substrings, with various configuration options.
            ///
            /// The builder struct offers methods to configure 3 dimensions, which map to different Godot functions in the back:
            ///
            /// | Method        | Default behavior                          | Behavior after method call         |
            /// |---------------|-------------------------------------------|------------------------------------|
            /// | `r()`         | forward search (`find*`)                  | backward search (`rfind*`)         |
            /// | `n()`         | case-sensitive search (`*find`)           | case-insensitive search (`*findn`) |
            /// | `from(index)` | search from beginning (or end if reverse) | search from specified index        |
            ///
            /// Returns `Some(index)` of the first occurrence, or `None` if not found.
            ///
            /// # Example
            /// To find the substring `"O"` in `"Hello World"`, not considering case and starting from position 5, you can write:
            /// ```no_run
            /// # use godot::prelude::*;
            /// # fn do_sth_with_index(_i: usize) {}
            /// let s = GString::from("Hello World");
            /// if let Some(found) = s.find_ex("O").n().from(5).done() {
            ///    do_sth_with_index(found)
            /// }
            /// ```
            /// This is equivalent to the following GDScript code:
            /// ```gdscript
            /// var s: GString = "Hello World"
            /// var found = s.findn("O", 5)
            /// if found != -1:
            ///     do_sth_with_index(found)
            /// ```
            #[doc(alias = "findn", alias = "rfind", alias = "rfindn")]
            pub fn find_ex<'s, 'w>(
                &'s self,
                what: impl AsArg<GString> + 'w,
            ) -> $FindBuilder<'s, 'w> {
                $FindBuilder::new(self, what.into_arg())
            }

            /// Count how many times `what` appears within `range`. Use `..` for full string search.
            pub fn count(&self, what: impl AsArg<GString>, range: impl std::ops::RangeBounds<usize>) -> usize {
                let (from, to) = $crate::meta::signed_range::to_godot_range_fromto(range);
                self.as_inner().count(what, from, to) as usize
            }

            /// Count how many times `what` appears within `range`, case-insensitively. Use `..` for full string search.
            pub fn countn(&self, what: impl AsArg<GString>, range: impl std::ops::RangeBounds<usize>) -> usize {
                let (from, to) = $crate::meta::signed_range::to_godot_range_fromto(range);
                self.as_inner().countn(what, from, to) as usize
            }

            /// Splits the string according to `delimiter`.
            ///
            /// See [`split_ex()`][Self::split_ex] if you need further configuration.
            pub fn split(&self, delimiter: impl AsArg<GString>) -> $crate::builtin::PackedStringArray {
                self.split_ex(delimiter).done()
            }

            /// Returns a builder that splits this string into substrings using `delimiter`.
            ///
            /// If `delimiter` is an empty string, each substring will be a single character.
            ///
            /// The builder struct offers methods to configure multiple dimensions. Note that `rsplit` in Godot is not useful without the
            /// `maxsplit` argument, so the two are combined in Rust as `maxsplit_r`.
            ///
            /// | Method             | Default behavior       | Behavior after method call                        |
            /// |--------------------|------------------------|---------------------------------------------------|
            /// | `disallow_empty()` | allows empty parts     | empty parts are removed from result               |
            /// | `maxsplit(n)`      | entire string is split | splits `n` times -> `n+1` parts                   |
            /// | `maxsplit_r(n)`    | entire string is split | splits `n` times -> `n+1` parts (start from back) |
            #[doc(alias = "rsplit")]
            pub fn split_ex<'s, 'w>(
                &'s self,
                delimiter: impl AsArg<GString> + 'w,
            ) -> $SplitBuilder<'s, 'w> {
                $SplitBuilder::new(self, delimiter.into_arg())
            }

            /// Returns a substring of this, as another `GString`.
            // TODO is there no efficient way to implement this for StringName by interning?
            pub fn substr(&self, range: impl std::ops::RangeBounds<usize>) -> GString {
                let (from, len) = $crate::meta::signed_range::to_godot_range_fromlen(range, -1);

                self.as_inner().substr(from, len)
            }

            /// Splits the string using a string delimiter and returns the substring at index `slice`.
            ///
            /// Returns the original string if delimiter does not occur in the string. Returns `None` if `slice` is out of bounds.
            ///
            /// This is faster than [`split()`][Self::split], if you only need one substring.
            pub fn get_slice(
                &self,
                delimiter: impl AsArg<GString>,
                slice: usize,
            ) -> Option<GString> {
                let sliced = self.as_inner().get_slice(delimiter, slice as i64);

                // Note: self="" always returns None.
                super::populated_or_none(sliced)
            }

            /// Splits the string using a Unicode char `delimiter` and returns the substring at index `slice`.
            ///
            /// Returns the original string if delimiter does not occur in the string. Returns `None` if `slice` is out of bounds.
            ///
            /// This is faster than [`split()`][Self::split], if you only need one substring.
            pub fn get_slicec(&self, delimiter: char, slice: usize) -> Option<GString> {
                let sliced = self.as_inner().get_slicec(delimiter as i64, slice as i64);

                // Note: self="" always returns None.
                super::populated_or_none(sliced)
            }

            /// Returns the total number of slices, when the string is split with the given delimiter.
            ///
            /// See also [`split()`][Self::split] and [`get_slice()`][Self::get_slice].
            pub fn get_slice_count(&self, delimiter: impl AsArg<GString>) -> usize {
                self.as_inner().get_slice_count(delimiter) as usize
            }

            /// Returns a copy of the string without the specified index range.
            pub fn erase(&self, range: impl std::ops::RangeBounds<usize>) -> GString {
                let (from, len) = $crate::meta::signed_range::to_godot_range_fromlen(range, i32::MAX as i64);
                self.as_inner().erase(from, len)
            }

            /// Returns a copy of the string with an additional string inserted at the given position.
            ///
            /// If the position is out of bounds, the string will be inserted at the end.
            ///
            /// Consider using [`format()`](Self::format) for more flexibility.
            pub fn insert(&self, position: usize, what: impl AsArg<GString>) -> GString {
                self.as_inner().insert(position as i64, what)
            }

            /// Format a string using substitutions from an array or dictionary.
            ///
            /// See Godot's [`String.format()`](https://docs.godotengine.org/en/stable/classes/class_string.html#class-string-method-format).
            pub fn format(&self, array_or_dict: &Variant) -> GString {
                self.as_inner().format(array_or_dict, "{_}")
            }

            /// Format a string using substitutions from an array or dictionary + custom placeholder.
            ///
            /// See Godot's [`String.format()`](https://docs.godotengine.org/en/stable/classes/class_string.html#class-string-method-format).
            pub fn format_with_placeholder(
                &self,
                array_or_dict: &Variant,
                placeholder: impl AsArg<GString>,
            ) -> GString {
                self.as_inner().format(array_or_dict, placeholder)
            }

            // left() + right() are not redefined, as their i64 can be negative.

            /// Formats the string to be at least `min_length` long, by adding characters to the left of the string, if necessary.
            ///
            /// Godot itself allows padding with multiple characters, but that behavior is not very useful, because `min_length` isn't
            /// respected in that case. The parameter in Godot is even called `character`. In Rust, we directly expose `char` instead.
            ///
            /// See also [`rpad()`](Self::rpad).
            pub fn lpad(&self, min_length: usize, character: char) -> GString {
                let one_char_string = GString::from([character].as_slice());
                self.as_inner().lpad(min_length as i64, &one_char_string)
            }

            /// Formats the string to be at least `min_length` long, by adding characters to the right of the string, if necessary.
            ///
            /// Godot itself allows padding with multiple characters, but that behavior is not very useful, because `min_length` isn't
            /// respected in that case. The parameter in Godot is even called `character`. In Rust, we directly expose `char` instead.
            ///
            /// See also [`lpad()`](Self::lpad).
            pub fn rpad(&self, min_length: usize, character: char) -> GString {
                let one_char_string = GString::from([character].as_slice());
                self.as_inner().rpad(min_length as i64, &one_char_string)
            }

            /// Formats the string representing a number to have an exact number of `digits` _after_ the decimal point.
            pub fn pad_decimals(&self, digits: usize) -> GString {
                self.as_inner().pad_decimals(digits as i64)
            }

            /// Formats the string representing a number to have an exact number of `digits` _before_ the decimal point.
            pub fn pad_zeros(&self, digits: usize) -> GString {
                self.as_inner().pad_zeros(digits as i64)
            }

            /// Case-sensitive, lexicographic comparison to another string.
            ///
            /// Returns the `Ordering` relation of `self` towards `to`. Ordering is determined by the Unicode code points of each string, which
            /// roughly matches the alphabetical order.
            ///
            /// See also [`nocasecmp_to()`](Self::nocasecmp_to), [`naturalcasecmp_to()`](Self::naturalcasecmp_to), [`filecasecmp_to()`](Self::filecasecmp_to).
            pub fn casecmp_to(&self, to: impl AsArg<GString>) -> std::cmp::Ordering {
                sys::i64_to_ordering(self.as_inner().casecmp_to(to))
            }

            /// Case-**insensitive**, lexicographic comparison to another string.
            ///
            /// Returns the `Ordering` relation of `self` towards `to`. Ordering is determined by the Unicode code points of each string, which
            /// roughly matches the alphabetical order.
            ///
            /// See also [`casecmp_to()`](Self::casecmp_to), [`naturalcasecmp_to()`](Self::naturalcasecmp_to), [`filecasecmp_to()`](Self::filecasecmp_to).
            pub fn nocasecmp_to(&self, to: impl AsArg<GString>) -> std::cmp::Ordering {
                sys::i64_to_ordering(self.as_inner().nocasecmp_to(to))
            }

            /// Case-sensitive, **natural-order** comparison to another string.
            ///
            /// Returns the `Ordering` relation of `self` towards `to`. Ordering is determined by the Unicode code points of each string, which
            /// roughly matches the alphabetical order.
            ///
            /// When used for sorting, natural order comparison orders sequences of numbers by the combined value of each digit as is often
            /// expected, instead of the single digit's value. A sorted sequence of numbered strings will be `["1", "2", "3", ...]`, not
            /// `["1", "10", "2", "3", ...]`.
            ///
            /// With different string lengths, returns `Ordering::Greater` if this string is longer than the `to` string, or `Ordering::Less`
            /// if shorter.
            ///
            /// See also [`casecmp_to()`](Self::casecmp_to), [`naturalnocasecmp_to()`](Self::naturalnocasecmp_to), [`filecasecmp_to()`](Self::filecasecmp_to).
            pub fn naturalcasecmp_to(&self, to: impl AsArg<GString>) -> std::cmp::Ordering {
                sys::i64_to_ordering(self.as_inner().naturalcasecmp_to(to))
            }

            /// Case-insensitive, **natural-order** comparison to another string.
            ///
            /// Returns the `Ordering` relation of `self` towards `to`. Ordering is determined by the Unicode code points of each string, which
            /// roughly matches the alphabetical order.
            ///
            /// When used for sorting, natural order comparison orders sequences of numbers by the combined value of each digit as is often
            /// expected, instead of the single digit's value. A sorted sequence of numbered strings will be `["1", "2", "3", ...]`, not
            /// `["1", "10", "2", "3", ...]`.
            ///
            /// With different string lengths, returns `Ordering::Greater` if this string is longer than the `to` string, or `Ordering::Less`
            /// if shorter.
            ///
            /// See also [`casecmp_to()`](Self::casecmp_to), [`naturalcasecmp_to()`](Self::naturalcasecmp_to), [`filecasecmp_to()`](Self::filecasecmp_to).
            pub fn naturalnocasecmp_to(&self, to: impl AsArg<GString>) -> std::cmp::Ordering {
                sys::i64_to_ordering(self.as_inner().naturalnocasecmp_to(to))
            }

            /// Case-sensitive, filename-oriented comparison to another string.
            ///
            /// Like [`naturalcasecmp_to()`][Self::naturalcasecmp_to], but prioritizes strings that begin with periods (`.`) and underscores
            /// (`_`) before any other character. Useful when sorting folders or file names.
            ///
            /// See also [`casecmp_to()`](Self::casecmp_to), [`naturalcasecmp_to()`](Self::naturalcasecmp_to), [`filenocasecmp_to()`](Self::filenocasecmp_to).
            #[cfg(since_api = "4.3")]
            pub fn filecasecmp_to(&self, to: impl AsArg<GString>) -> std::cmp::Ordering {
                sys::i64_to_ordering(self.as_inner().filecasecmp_to(to))
            }

            /// Case-insensitive, filename-oriented comparison to another string.
            ///
            /// Like [`naturalnocasecmp_to()`][Self::naturalnocasecmp_to], but prioritizes strings that begin with periods (`.`) and underscores
            /// (`_`) before any other character. Useful when sorting folders or file names.
            ///
            /// See also [`casecmp_to()`](Self::casecmp_to), [`naturalcasecmp_to()`](Self::naturalcasecmp_to), [`filecasecmp_to()`](Self::filecasecmp_to).
            #[cfg(since_api = "4.3")]
            pub fn filenocasecmp_to(&self, to: impl AsArg<GString>) -> std::cmp::Ordering {
                sys::i64_to_ordering(self.as_inner().filenocasecmp_to(to))
            }

            /// Simple expression match (also called "glob" or "globbing"), where `*` matches zero or more arbitrary characters and `?`
            /// matches any single character except a period (`.`).
            ///
            /// An empty string or empty expression always evaluates to `false`.
            ///
            /// Renamed from `match` because of collision with Rust keyword + possible confusion with `String::matches()` that can match regex.
            #[doc(alias = "match")]
            pub fn match_glob(&self, pattern: impl AsArg<GString>) -> bool {
                self.as_inner().match_(pattern)
            }

            /// Simple **case-insensitive** expression match (also called "glob" or "globbing"), where `*` matches zero or more arbitrary
            /// characters and `?` matches any single character except a period (`.`).
            ///
            /// An empty string or empty expression always evaluates to `false`.
            ///
            /// Renamed from `matchn` because of collision with Rust keyword + possible confusion with `String::matches()` that can match regex.
            #[doc(alias = "matchn")]
            pub fn matchn_glob(&self, pattern: impl AsArg<GString>) -> bool {
                self.as_inner().matchn(pattern)
            }
        }

        // --------------------------------------------------------------------------------------------------------------------------------------
        // find() support

        #[doc = concat!("Builder for [`", stringify!($Builtin), "::find_ex()`][", stringify!($Builtin), "::find_ex].")]
        #[must_use]
        pub struct $FindBuilder<'s, 'w> {
            owner: &'s $Builtin,
            what: meta::CowArg<'w, GString>,
            reverse: bool,
            case_insensitive: bool,
            from_index: Option<usize>,
        }

        impl<'s, 'w> $FindBuilder<'s, 'w> {
            pub(crate) fn new(owner: &'s $Builtin, what: meta::CowArg<'w, GString>) -> Self {
                Self {
                    owner,
                    what,
                    reverse: false,
                    case_insensitive: false,
                    from_index: None,
                }
            }

            /// Reverse search direction (start at back).
            pub fn r(self) -> Self {
                Self {
                    reverse: true,
                    ..self
                }
            }

            /// Case-**insensitive** search.
            pub fn n(self) -> Self {
                Self {
                    case_insensitive: true,
                    ..self
                }
            }

            /// Start index -- begin search here rather than at start/end of string.
            pub fn from(self, index: usize) -> Self {
                Self {
                    from_index: Some(index),
                    ..self
                }
            }

            /// Does the actual work. Must be called to finalize find operation.
            pub fn done(self) -> Option<usize> {
                let from_index = self.from_index.map(|i| i as i64);
                let inner = self.owner.as_inner();
                let what = self.what;

                let godot_found = if self.reverse {
                    let from_index = from_index.unwrap_or(-1);

                    if self.case_insensitive {
                        inner.rfindn(what, from_index)
                    } else {
                        inner.rfind(what, from_index)
                    }
                } else {
                    let from_index = from_index.unwrap_or(0);

                    if self.case_insensitive {
                        inner.findn(what, from_index)
                    } else {
                        inner.find(what, from_index)
                    }
                };

                sys::found_to_option(godot_found)
            }
        }

        // --------------------------------------------------------------------------------------------------------------------------------------
        // split() support

        #[doc = concat!("Builder for [`", stringify!($Builtin), "::split_ex()`][", stringify!($Builtin), "::split_ex].")]
        #[must_use]
        pub struct $SplitBuilder<'s, 'w> {
            owner: &'s $Builtin,
            delimiter: meta::CowArg<'w, GString>,
            reverse: bool,
            allow_empty: bool,
            maxsplit: usize,
        }

        impl<'s, 'w> $SplitBuilder<'s, 'w> {
            pub(crate) fn new(owner: &'s $Builtin, delimiter: meta::CowArg<'w, GString>) -> Self {
                Self {
                    owner,
                    delimiter,
                    reverse: false,
                    allow_empty: true,
                    maxsplit: 0,
                }
            }

            /// After calling this method, empty strings between adjacent delimiters are excluded from the array.
            pub fn disallow_empty(self) -> Self {
                Self {
                    allow_empty: false,
                    ..self
                }
            }

            /// Limit number of splits (forward mode).
            ///
            /// If `maxsplit` is greater than 0, the number of splits may not exceed `maxsplit`. By default, the entire string is split.
            ///
            /// Note that `number_of_splits` refers to the number of times a split occurs, which is the resulting element count **minus one**.
            pub fn maxsplit(self, number_of_splits: usize) -> Self {
                Self {
                    maxsplit: number_of_splits,
                    ..self
                }
            }

            /// Limit number of splits (reverse mode).
            ///
            /// If `maxsplit` is greater than 0, the number of splits may not exceed `maxsplit`. By default, the entire string is split.
            ///
            /// Note that `number_of_splits` refers to the number of times a split occurs, which is the resulting element count **minus one**.
            pub fn maxsplit_r(self, number_of_splits: usize) -> Self {
                Self {
                    maxsplit: number_of_splits,
                    reverse: true,
                    ..self
                }
            }

            /// Does the actual work. Must be called to finalize split operation.
            pub fn done(self) -> $crate::builtin::PackedStringArray {
                let inner = self.owner.as_inner();
                let delimiter = self.delimiter;

                if self.reverse {
                    inner.rsplit(delimiter, self.allow_empty, self.maxsplit as i64)
                } else {
                    inner.split(delimiter, self.allow_empty, self.maxsplit as i64)
                }
            }
        }
    };
}
