/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use godot::prelude::*;

/// *documented* ~ **documented** ~ [AABB] [pr](https://github.com/godot-rust/gdext/pull/748)
///
/// a few tests:
///
/// headings:
///
/// # Some heading
///
/// lists:
///
/// - lists
/// - like this
/// * maybe with `*` as well
///
/// links with back-references:
///
/// Blah blah [^foo]
/// [^foo]: https://example.org
///
/// footnotes:
///
/// We cannot florbinate the glorb[^florb]
/// [^florb]: because the glorb doesn't flibble.
///
/// task lists:
///
/// We must ensure that we've completed
/// - [ ] task 1
/// - [x] task 2
///
/// tables:
///
/// | Header1 | Header2 |
/// |---------|---------|
/// | abc     | def     |
///
/// images:
///
/// ![Image](http://url/a.png)
///
/// blockquotes:
///
/// > Some cool thing
///
/// ordered list:
///
/// 1. thing one
/// 2. thing two
///
///
/// Something here < this is technically header syntax
/// ---
/// And here
///
/// smart punctuation
///
/// codeblocks:
///
/// ```rust
/// #![no_main]
/// #[link_section=".text"]
/// #[no_mangle]
/// static main: u64 = 0x31c0678b10;
/// ```
///
/// connect
/// these
#[derive(GodotClass)]
#[class(base=Node)]
pub struct ExtremelyDocumented {
    #[doc = r#"this is very documented"#]
    #[var]
    item: f32,
    /// this isnt documented
    _other_item: (),
    /// nor this
    base: Base<Node>,
}

#[godot_api]
impl INode for ExtremelyDocumented {
    /// initialize this
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            item: 883.0,
            _other_item: {},
        }
    }
}

#[godot_api]
impl ExtremelyDocumented {
    #[constant]
    /// Documentation.
    const RANDOM: i64 = 4;

    #[func]
    /// huh
    fn ye(&self) -> f32 {
        self.item
    }

    #[func]
    /// wow
    fn ne(_x: f32) -> Gd<Self> {
        panic!()
    }
}

#[test]
#[cfg(feature = "register-docs")]
fn correct() {
    // Uncomment if implementation changes and expected output file should be rewritten.
    // std::fs::write(
    //     "tests/docs.xml",
    //     godot_core::docs::gather_xml_docs().next().unwrap(),
    // );
    assert_eq!(
        include_str!("docs.xml"),
        godot_core::docs::gather_xml_docs().next().unwrap()
    );
}
