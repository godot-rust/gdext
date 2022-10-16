/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::itest;
use godot_core::api::{file_access, ip, os};
use std::collections::HashSet;

pub fn run() -> bool {
    let mut ok = true;
    ok &= enum_ords_correct();
    ok &= enum_equality();
    ok &= enum_hash();
    ok
}

#[itest]
fn enum_ords_correct() {
    assert_eq!(ip::ResolverStatus::RESOLVER_STATUS_NONE.ord(), 0);
    assert_eq!(ip::ResolverStatus::RESOLVER_STATUS_WAITING.ord(), 1);
    assert_eq!(ip::ResolverStatus::RESOLVER_STATUS_DONE.ord(), 2);
    assert_eq!(ip::ResolverStatus::RESOLVER_STATUS_ERROR.ord(), 3);
}

#[itest]
fn enum_equality() {
    // TODO: find 2 overlapping ords in same enum

    assert_eq!(
        file_access::CompressionMode::COMPRESSION_DEFLATE,
        file_access::CompressionMode::COMPRESSION_DEFLATE
    );
}

#[itest]
fn enum_hash() {
    let mut months = HashSet::new();
    months.insert(os::Month::MONTH_JANUARY);
    months.insert(os::Month::MONTH_FEBRUARY);
    months.insert(os::Month::MONTH_MARCH);
    months.insert(os::Month::MONTH_APRIL);
    months.insert(os::Month::MONTH_MAY);
    months.insert(os::Month::MONTH_JUNE);
    months.insert(os::Month::MONTH_JULY);
    months.insert(os::Month::MONTH_AUGUST);
    months.insert(os::Month::MONTH_SEPTEMBER);
    months.insert(os::Month::MONTH_OCTOBER);
    months.insert(os::Month::MONTH_NOVEMBER);
    months.insert(os::Month::MONTH_DECEMBER);

    assert_eq!(months.len(), 12);
}
