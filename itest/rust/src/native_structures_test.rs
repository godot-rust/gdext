/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::itest;
use godot::engine::text_server::Direction;
use godot::engine::{TextServer, TextServerExtension, TextServerExtensionVirtual};
use godot::native_structure::{AudioFrame, CaretInfo, Glyph};
use godot::prelude::{godot_api, Base, Gd, GodotClass, Rect2, Rid, Share, Variant};

use std::cell::Cell;

#[derive(GodotClass)]
#[class(base=TextServerExtension)]
pub struct TestTextServer {
    #[base]
    base: Base<TextServerExtension>,
    glyphs: [Glyph; 2],
    cell: Cell<Option<(Rid, i64)>>,
}

// Simple function to make these up with one field differing.
fn sample_glyph(start: i32) -> Glyph {
    Glyph {
        start,
        end: 8,
        count: 9,
        repeat: 10,
        flags: 33,
        x_off: 999.0,
        y_off: -999.0,
        advance: 132.0,
        font_rid: Rid::new(1024),
        font_size: 1025,
        index: 1026,
    }
}

#[godot_api]
impl TextServerExtensionVirtual for TestTextServer {
    fn init(base: Base<TextServerExtension>) -> Self {
        TestTextServer {
            base,
            glyphs: [sample_glyph(99), sample_glyph(700)],
            cell: Cell::new(None),
        }
    }

    unsafe fn shaped_text_get_carets(&self, shaped: Rid, position: i64, caret: *mut CaretInfo) {
        // Record the arguments we were called with.
        self.cell.set(Some((shaped, position)));
        // Now put something in the out param.
        *caret = CaretInfo {
            leading_caret: Rect2::from_components(0.0, 0.0, 0.0, 0.0),
            trailing_caret: Rect2::from_components(1.0, 1.0, 1.0, 1.0),
            leading_direction: Direction::DIRECTION_AUTO,
            trailing_direction: Direction::DIRECTION_LTR,
        };
    }

    fn shaped_text_get_glyph_count(&self, _shaped: Rid) -> i64 {
        self.glyphs.len() as i64
    }

    unsafe fn shaped_text_get_glyphs(&self, _shaped: Rid) -> *const Glyph {
        self.glyphs.as_ptr()
    }
}

#[itest]
fn test_native_structures_codegen() {
    // Test construction of a few simple types.
    let _ = AudioFrame {
        left: 0.0,
        right: 0.0,
    };
    let _ = Glyph {
        start: 0,
        end: 0,
        count: 0,
        repeat: 0,
        flags: 0,
        x_off: 0.0,
        y_off: 0.0,
        advance: 0.0,
        font_rid: Rid::new(0),
        font_size: 0,
        index: 0,
    };
}

#[itest]
fn test_native_structure_out_parameter() {
    // Instantiate a TextServerExtension and then have Godot call a
    // function which uses an 'out' pointer parameter.
    let mut ext: Gd<TestTextServer> = Gd::new_default();
    let result = ext
        .share()
        .upcast::<TextServer>()
        .shaped_text_get_carets(Rid::new(100), 200);

    // Check that we called the virtual function.
    let cell = ext.bind_mut().cell.take();
    assert_eq!(cell, Some((Rid::new(100), 200)));

    // Check the result dictionary (Godot made it out of our 'out'
    // param).
    assert_eq!(
        result.get("leading_rect"),
        Some(Variant::from(Rect2::from_components(0.0, 0.0, 0.0, 0.0)))
    );
    assert_eq!(
        result.get("trailing_rect"),
        Some(Variant::from(Rect2::from_components(1.0, 1.0, 1.0, 1.0)))
    );
    assert_eq!(
        result.get("leading_direction"),
        Some(Variant::from(Direction::DIRECTION_AUTO))
    );
    assert_eq!(
        result.get("trailing_direction"),
        Some(Variant::from(Direction::DIRECTION_LTR))
    );
}

#[itest]
fn test_native_structure_pointer_to_array_parameter() {
    // Instantiate a TextServerExtension.
    let ext: Gd<TestTextServer> = Gd::new_default();
    let result = ext
        .share()
        .upcast::<TextServer>()
        .shaped_text_get_glyphs(Rid::new(100));

    // Check the result array.
    assert_eq!(result.len(), 2);
    assert_eq!(result.get(0).get("start"), Some(Variant::from(99)));
    assert_eq!(result.get(1).get("start"), Some(Variant::from(700)));
}
