/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Byte and C-string conversions.

#[macro_export]
macro_rules! generate_string_bytes_and_cstr_tests {
    (
        builtin: $T:ty,
        tests: [
            $from_bytes_ascii:ident,
            $from_cstr_ascii:ident,
            $from_bytes_latin1:ident,
            $from_cstr_latin1:ident,
            $from_bytes_utf8:ident,
            $from_cstr_utf8:ident,
        ]
    ) => {
        #[itest]
        fn $from_bytes_ascii() {
            let ascii = <$T>::try_from_bytes(b"Hello", Encoding::Ascii).expect("valid ASCII");
            assert_eq!(ascii, <$T>::from("Hello"));
            assert_eq!(ascii.len(), 5);

            let ascii_nul = <$T>::try_from_bytes(b"Hello\0", Encoding::Ascii);
            assert_eq!(ascii_nul, None, "intermediate NUL byte is not valid ASCII"); // at end, but still not NUL terminator.

            let latin1 = <$T>::try_from_bytes(b"/\xF0\xF5\xBE", Encoding::Ascii);
            assert_eq!(latin1, None, "Latin-1 is *not* valid ASCII");

            let utf8 =
                <$T>::try_from_bytes(b"\xF6\xF0\x9F\x8D\x8E\xF0\x9F\x92\xA1", Encoding::Ascii);
            assert_eq!(utf8, None, "UTF-8 is *not* valid ASCII");
        }

        #[itest]
        fn $from_cstr_ascii() {
            let ascii = <$T>::try_from_cstr(c"Hello", Encoding::Ascii);
            let ascii = ascii.expect("valid ASCII");
            assert_eq!(ascii, <$T>::from("Hello"));
            assert_eq!(ascii.len(), 5);

            let latin1 = <$T>::try_from_cstr(c"/ðõ¾", Encoding::Ascii);
            assert_eq!(latin1, None, "Latin-1 is *not* valid ASCII");

            let utf8 = <$T>::try_from_cstr(c"ö🍎A💡", Encoding::Ascii);
            assert_eq!(utf8, None, "UTF-8 is *not* valid ASCII");
        }

        #[itest]
        fn $from_bytes_latin1() {
            let ascii = <$T>::try_from_bytes(b"Hello", Encoding::Latin1);
            let ascii = ascii.expect("ASCII is valid Latin-1");
            assert_eq!(ascii, <$T>::from("Hello"));
            assert_eq!(ascii.len(), 5);

            let latin1 = <$T>::try_from_bytes(b"/\xF0\xF5\xBE", Encoding::Latin1);
            let latin1 = latin1.expect("Latin-1 is valid Latin-1");
            assert_eq!(latin1, <$T>::from("/ðõ¾"));
            assert_eq!(latin1.len(), 4);

            let latin1_nul = <$T>::try_from_bytes(b"/\0\xF0\xF5\xBE", Encoding::Latin1);
            assert_eq!(
                latin1_nul, None,
                "intermediate NUL byte is not valid Latin-1"
            );

            // UTF-8 -> Latin-1: always succeeds, even if result is garbage, since every byte is a valid Latin-1 character.
            let utf8 = <$T>::try_from_bytes(
                b"\xC3\xB6\xF0\x9F\x8D\x8E\x41\xF0\x9F\x92\xA1",
                Encoding::Latin1,
            );
            let utf8 = utf8.expect("UTF-8 is valid Latin-1, even if garbage");
            assert_eq!(utf8, <$T>::from("Ã¶ðAð¡"));
        }

        #[itest]
        fn $from_cstr_latin1() {
            let ascii = <$T>::try_from_cstr(c"Hello", Encoding::Latin1);
            let ascii = ascii.expect("ASCII is valid Latin-1");
            assert_eq!(ascii, <$T>::from("Hello"));
            assert_eq!(ascii.len(), 5);

            // The C-string literal is interpreted as UTF-8, not Latin-1 (which is btw still valid Latin-1), see last test in this #[itest].
            // So we use explicit bytes in the following tests.
            assert_eq!(c"/ðõ¾".to_bytes(), b"/\xC3\xB0\xC3\xB5\xC2\xBE");
            let latin1 = <$T>::try_from_cstr(c"/\xF0\xF5\xBE", Encoding::Latin1);
            let latin1 = latin1.expect("Latin-1 is valid Latin-1");
            assert_eq!(latin1, <$T>::from("/ðõ¾"));
            assert_eq!(latin1.len(), 4);

            // UTF-8 -> Latin-1: always succeeds, even if result is garbage, since every byte is a valid Latin-1 character.
            let utf8 = <$T>::try_from_cstr(c"ö🍎A💡", Encoding::Latin1);
            let utf8 = utf8.expect("UTF-8 is valid Latin-1, even if garbage");
            assert_eq!(utf8, <$T>::from("Ã¶ðAð¡"));
        }

        #[itest]
        fn $from_bytes_utf8() {
            let ascii = <$T>::try_from_bytes(b"Hello", Encoding::Utf8);
            let ascii = ascii.expect("ASCII is valid UTF-8");
            assert_eq!(ascii, <$T>::from("Hello"));
            assert_eq!(ascii.len(), 5);

            let latin1 = <$T>::try_from_bytes(b"/\xF0\xF5\xBE", Encoding::Utf8);
            assert_eq!(latin1, None, "Latin-1 is *not* valid UTF-8");

            let utf8 = <$T>::try_from_bytes(
                b"\xC3\xB6\xF0\x9F\x8D\x8E\x41\xF0\x9F\x92\xA1",
                Encoding::Utf8,
            );
            let utf8 = utf8.expect("UTF-8 is valid UTF-8");
            assert_eq!(utf8, <$T>::from("ö🍎A💡"));
            assert_eq!(utf8.len(), 4);

            let utf8_nul = <$T>::try_from_bytes(b"\xC3\0A", Encoding::Utf8);
            assert_eq!(utf8_nul, None, "intermediate NUL byte is not valid UTF-8");
        }

        #[itest]
        fn $from_cstr_utf8() {
            let ascii = <$T>::try_from_cstr(c"Hello", Encoding::Utf8);
            let ascii = ascii.expect("ASCII is valid UTF-8");
            assert_eq!(ascii, <$T>::from("Hello"));
            assert_eq!(ascii.len(), 5);

            // The latin1 checks pass even though try_from_bytes() for the Latin-1 string b"/\xF0\xF5\xBE" fails.
            // When using a C string literal, the characters are interpreted as UTF-8, *not* Latin-1, see following assertion.
            assert_eq!(c"/ðõ¾".to_bytes(), b"/\xC3\xB0\xC3\xB5\xC2\xBE");
            let latin1 = <$T>::try_from_cstr(c"/ðõ¾", Encoding::Utf8);
            let latin1 =
                latin1.expect("Characters from Latin-1 set re-encoded as UTF-8 are valid UTF-8");
            assert_eq!(latin1, <$T>::from("/ðõ¾"));
            assert_eq!(latin1.len(), 4);

            let utf8 = <$T>::try_from_cstr(c"ö🍎A💡", Encoding::Utf8);
            let utf8 = utf8.expect("valid UTF-8");
            assert_eq!(utf8, <$T>::from("ö🍎A💡"));
            assert_eq!(utf8.len(), 4);
        }
    };
}
