/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use super::*;

#[derive(Debug, Default, Clone, PartialEq, ToVariant, FromVariant)]
enum EnumWithSkip {
    #[variant(skip)]
    Skipped(String),
    NewType(#[variant(skip)] String),
    PartSkippedTuple(#[variant(skip)] String, String),
    PartSkippedNamed {
        #[variant(skip)]
        skipped_data: String,
        data: String,
    },
    #[default]
    Default,
}

roundtrip_with_skip!(
    skipped_to_variant,
    skipped_from_variant,
    EnumWithSkip::Skipped("one".to_string()),
    dict! { "EnumWithSkip" : Variant::nil() },
    EnumWithSkip::default()
);

roundtrip_with_skip!(
    skipped_newtype_to_variant,
    skipped_newtype_from_variant,
    EnumWithSkip::NewType("whatever".to_string()),
    dict! { "EnumWithSkip" : dict!{ "NewType" : Variant::nil() } },
    EnumWithSkip::NewType(String::default())
);

roundtrip_with_skip!(
    skipped_tuple_to_variant,
    skipped_tuple_from_variant,
    EnumWithSkip::PartSkippedTuple("skipped".to_string(), "three".to_string()),
    dict! {
        "EnumWithSkip": dict!{
            "PartSkippedTuple" : varray!["three"]
        }
    },
    EnumWithSkip::PartSkippedTuple(String::default(), "three".to_string())
);

roundtrip_with_skip!(
    named_skipped_to_variant,
    named_skipped_from_variant,
    EnumWithSkip::PartSkippedNamed {
        skipped_data: "four".to_string(),
        data: "five".to_string(),
    },
    dict! {
        "EnumWithSkip": dict!{
            "PartSkippedNamed" : dict! { "data" : "five" }
        }
    },
    EnumWithSkip::PartSkippedNamed {
        data: "five".to_string(),
        skipped_data: String::default()
    }
);
