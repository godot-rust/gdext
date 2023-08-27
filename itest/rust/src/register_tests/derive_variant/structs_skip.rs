/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::*;

#[derive(Debug, Default, PartialEq, ToVariant, FromVariant)]
struct NewTypeStructWithSkip(#[variant(skip)] String);

#[derive(Debug, Default, PartialEq, ToVariant, FromVariant)]
struct StructWithSkip {
    #[variant(skip)]
    skipped_field: String,
    field: String,
}

#[itest]
fn new_type_to_variant() {
    assert_eq!(
        NewTypeStructWithSkip("four".to_string()).to_variant(),
        dict! {"NewTypeStructWithSkip" : varray![] }.to_variant()
    );
}

#[itest]
fn new_type_from_variant() {
    let s = NewTypeStructWithSkip("four".to_string());
    assert_eq!(
        NewTypeStructWithSkip::from_variant(&s.to_variant()),
        NewTypeStructWithSkip::default()
    )
}

#[itest]
fn struct_with_skip_to_variant() {
    assert_eq!(
        StructWithSkip {
            skipped_field: "four".to_string(),
            field: "seven".to_string(),
        }
        .to_variant(),
        dict! { "StructWithSkip" : dict! { "field" : "seven" } }.to_variant()
    );
}

#[itest]
fn struct_with_skip_from_variant() {
    assert_eq!(
        StructWithSkip {
            field: "seven".to_string(),
            ..Default::default()
        },
        StructWithSkip::from_variant(
            &StructWithSkip {
                skipped_field: "four".to_string(),
                field: "seven".to_string(),
            }
            .to_variant()
        )
    );
}
