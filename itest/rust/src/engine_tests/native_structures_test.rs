/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::framework::itest;

use godot::builtin::{
    Array, Color, Dictionary, GString, PackedInt32Array, Rect2, Rid, Variant, VariantArray,
    Vector2, Vector2i,
};
use godot::classes::native::{
    AudioFrame, CaretInfo, Glyph, ObjectId, PhysicsServer2DExtensionShapeResult,
};
use godot::classes::text_server::Direction;
use godot::classes::{ITextServerExtension, Node3D, RefCounted, TextServer, TextServerExtension};
use godot::engine::{text_server, Image};
use godot::global::InlineAlignment;
use godot::obj::{Base, Gd, NewAlloc, NewGd};
use godot::register::{godot_api, GodotClass};

use std::cell::Cell;

#[derive(GodotClass)]
#[class(base=TextServerExtension)]
pub struct TestTextServer {
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
impl ITextServerExtension for TestTextServer {
    fn init(_base: Base<TextServerExtension>) -> Self {
        TestTextServer {
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
            leading_direction: Direction::AUTO,
            trailing_direction: Direction::LTR,
        };
    }

    fn shaped_text_get_glyph_count(&self, _shaped: Rid) -> i64 {
        self.glyphs.len() as i64
    }

    unsafe fn shaped_text_get_glyphs(&self, _shaped: Rid) -> *const Glyph {
        self.glyphs.as_ptr()
    }

    fn has_feature(&self, _feature: text_server::Feature) -> bool {
        unreachable!()
    }

    fn get_name(&self) -> GString {
        unreachable!()
    }

    fn get_features(&self) -> i64 {
        unreachable!()
    }

    fn free_rid(&mut self, _rid: Rid) {
        unreachable!()
    }

    fn has(&mut self, _rid: Rid) -> bool {
        unreachable!()
    }

    fn create_font(&mut self) -> Rid {
        unreachable!()
    }

    fn font_set_fixed_size(&mut self, _font_rid: Rid, _fixed_sizee: i64) {
        unreachable!()
    }

    fn font_get_fixed_size(&self, _font_rid: Rid) -> i64 {
        unreachable!()
    }

    fn font_set_fixed_size_scale_mode(
        &mut self,
        _font_rid: Rid,
        _fixed_size_scale_mode: text_server::FixedSizeScaleMode,
    ) {
        unreachable!()
    }

    fn font_get_fixed_size_scale_mode(&self, _font_rid: Rid) -> text_server::FixedSizeScaleMode {
        unreachable!()
    }

    fn font_get_size_cache_list(&self, _font_rid: Rid) -> Array<Vector2i> {
        unreachable!()
    }

    fn font_clear_size_cache(&mut self, _font_rid: Rid) {
        unreachable!()
    }

    fn font_remove_size_cache(&mut self, _font_rid: Rid, _size: Vector2i) {
        unreachable!()
    }

    fn font_set_ascent(&mut self, _font_rid: Rid, _size: i64, _ascent: f64) {
        unreachable!()
    }

    fn font_get_ascent(&self, _font_rid: Rid, _size: i64) -> f64 {
        unreachable!()
    }

    fn font_set_descent(&mut self, _font_rid: Rid, _size: i64, _descent: f64) {
        unreachable!()
    }

    fn font_get_descent(&self, _font_rid: Rid, _size: i64) -> f64 {
        unreachable!()
    }

    fn font_set_underline_position(
        &mut self,
        _font_rid: Rid,
        _size: i64,
        _underline_position: f64,
    ) {
        unreachable!()
    }

    fn font_get_underline_position(&self, _font_rid: Rid, _size: i64) -> f64 {
        unreachable!()
    }

    fn font_set_underline_thickness(
        &mut self,
        _font_rid: Rid,
        _size: i64,
        _underline_thickness: f64,
    ) {
        unreachable!()
    }

    fn font_get_underline_thickness(&self, _font_rid: Rid, _size: i64) -> f64 {
        unreachable!()
    }

    fn font_set_scale(&mut self, _font_rid: Rid, _size: i64, _scale: f64) {
        unreachable!()
    }

    fn font_get_scale(&self, _font_rid: Rid, _size: i64) -> f64 {
        unreachable!()
    }

    fn font_get_texture_count(&self, _font_rid: Rid, _size: Vector2i) -> i64 {
        unreachable!()
    }

    fn font_clear_textures(&mut self, _font_rid: Rid, _size: Vector2i) {
        unreachable!()
    }

    fn font_remove_texture(&mut self, _font_rid: Rid, _size: Vector2i, _texture_index: i64) {
        unreachable!()
    }

    fn font_set_texture_image(
        &mut self,
        _font_rid: Rid,
        _size: Vector2i,
        _texture_index: i64,
        _image: Gd<Image>,
    ) {
        unreachable!()
    }

    fn font_get_texture_image(
        &self,
        _font_rid: Rid,
        _size: Vector2i,
        _texture_index: i64,
    ) -> Option<Gd<Image>> {
        unreachable!()
    }

    fn font_get_glyph_list(&self, _font_rid: Rid, _size: Vector2i) -> PackedInt32Array {
        unreachable!()
    }

    fn font_clear_glyphs(&mut self, _font_rid: Rid, _size: Vector2i) {
        unreachable!()
    }

    fn font_remove_glyph(&mut self, _font_rid: Rid, _size: Vector2i, _glyph: i64) {
        unreachable!()
    }

    fn font_get_glyph_advance(&self, _font_rid: Rid, _size: i64, _glyph: i64) -> Vector2 {
        unreachable!()
    }

    fn font_set_glyph_advance(
        &mut self,
        _font_rid: Rid,
        _size: i64,
        _glyph: i64,
        _advance: Vector2,
    ) {
        unreachable!()
    }

    fn font_get_glyph_offset(&self, _font_rid: Rid, _size: Vector2i, _glyph: i64) -> Vector2 {
        unreachable!()
    }

    fn font_set_glyph_offset(
        &mut self,
        _font_rid: Rid,
        _size: Vector2i,
        _glyph: i64,
        _offset: Vector2,
    ) {
        unreachable!()
    }

    fn font_get_glyph_size(&self, _font_rid: Rid, _size: Vector2i, _glyph: i64) -> Vector2 {
        unreachable!()
    }

    fn font_set_glyph_size(
        &mut self,
        _font_rid: Rid,
        _size: Vector2i,
        _glyph: i64,
        _gl_size: Vector2,
    ) {
        unreachable!()
    }

    fn font_get_glyph_uv_rect(&self, _font_rid: Rid, _size: Vector2i, _glyph: i64) -> Rect2 {
        unreachable!()
    }

    fn font_set_glyph_uv_rect(
        &mut self,
        _font_rid: Rid,
        _size: Vector2i,
        _glyph: i64,
        _uv_rect: Rect2,
    ) {
        unreachable!()
    }

    fn font_get_glyph_texture_idx(&self, _font_rid: Rid, _size: Vector2i, _glyph: i64) -> i64 {
        unreachable!()
    }

    fn font_set_glyph_texture_idx(
        &mut self,
        _font_rid: Rid,
        _size: Vector2i,
        _glyph: i64,
        _texture_idx: i64,
    ) {
        unreachable!()
    }

    fn font_get_glyph_texture_rid(&self, _font_rid: Rid, _size: Vector2i, _glyph: i64) -> Rid {
        unreachable!()
    }

    fn font_get_glyph_texture_size(&self, _font_rid: Rid, _size: Vector2i, _glyph: i64) -> Vector2 {
        unreachable!()
    }

    fn font_get_glyph_index(
        &self,
        _font_rid: Rid,
        _size: i64,
        _char: i64,
        _variation_selector: i64,
    ) -> i64 {
        unreachable!()
    }

    fn font_get_char_from_glyph_index(&self, _font_rid: Rid, _size: i64, _glyph_index: i64) -> i64 {
        unreachable!()
    }

    fn font_has_char(&self, _font_rid: Rid, _char: i64) -> bool {
        unreachable!()
    }

    fn font_get_supported_chars(&self, _font_rid: Rid) -> GString {
        unreachable!()
    }

    fn font_draw_glyph(
        &self,
        _font_rid: Rid,
        _canvas: Rid,
        _size: i64,
        _pos: Vector2,
        _index: i64,
        _color: Color,
    ) {
        unreachable!()
    }

    fn font_draw_glyph_outline(
        &self,
        _font_rid: Rid,
        _canvas: Rid,
        _size: i64,
        _outline_size: i64,
        _pos: Vector2,
        _index: i64,
        _color: Color,
    ) {
        unreachable!()
    }

    fn create_shaped_text(
        &mut self,
        _direction: text_server::Direction,
        _orientation: text_server::Orientation,
    ) -> Rid {
        unreachable!()
    }

    fn shaped_text_clear(&mut self, _shaped: Rid) {
        unreachable!()
    }

    fn shaped_text_add_string(
        &mut self,
        _shaped: Rid,
        _text: GString,
        _fonts: Array<Rid>,
        _size: i64,
        _opentype_features: Dictionary,
        _language: GString,
        _meta: Variant,
    ) -> bool {
        unreachable!()
    }

    fn shaped_text_add_object(
        &mut self,
        _shaped: Rid,
        _key: Variant,
        _size: Vector2,
        _inline_align: InlineAlignment,
        _length: i64,
        _baseline: f64,
    ) -> bool {
        unreachable!()
    }

    fn shaped_text_resize_object(
        &mut self,
        _shaped: Rid,
        _key: Variant,
        _size: Vector2,
        _inline_align: InlineAlignment,
        _baseline: f64,
    ) -> bool {
        unreachable!()
    }

    fn shaped_get_span_count(&self, _shaped: Rid) -> i64 {
        unreachable!()
    }

    fn shaped_get_span_meta(&self, _shaped: Rid, _index: i64) -> Variant {
        unreachable!()
    }

    fn shaped_set_span_update_font(
        &mut self,
        _shaped: Rid,
        _index: i64,
        _fonts: Array<Rid>,
        _size: i64,
        _opentype_features: Dictionary,
    ) {
        unreachable!()
    }

    fn shaped_text_substr(&self, _shaped: Rid, _start: i64, _lengthh: i64) -> Rid {
        unreachable!()
    }

    fn shaped_text_get_parent(&self, _shaped: Rid) -> Rid {
        unreachable!()
    }

    fn shaped_text_shape(&mut self, _shaped: Rid) -> bool {
        unreachable!()
    }

    fn shaped_text_is_ready(&self, _shaped: Rid) -> bool {
        unreachable!()
    }

    unsafe fn shaped_text_sort_logical(&mut self, _shaped: Rid) -> *const Glyph {
        unreachable!()
    }

    fn shaped_text_get_range(&self, _shaped: Rid) -> Vector2i {
        unreachable!()
    }

    fn shaped_text_get_trim_pos(&self, _shaped: Rid) -> i64 {
        unreachable!()
    }

    fn shaped_text_get_ellipsis_pos(&self, _shaped: Rid) -> i64 {
        unreachable!()
    }

    fn shaped_text_get_ellipsis_glyph_count(&self, _shaped: Rid) -> i64 {
        unreachable!()
    }

    unsafe fn shaped_text_get_ellipsis_glyphs(&self, _shaped: Rid) -> *const Glyph {
        unreachable!()
    }

    fn shaped_text_get_objects(&self, _shaped: Rid) -> VariantArray {
        unreachable!()
    }

    fn shaped_text_get_object_rect(&self, _shaped: Rid, _key: Variant) -> Rect2 {
        unreachable!()
    }

    fn shaped_text_get_size(&self, _shaped: Rid) -> Vector2 {
        unreachable!()
    }

    fn shaped_text_get_ascent(&self, _shaped: Rid) -> f64 {
        unreachable!()
    }

    fn shaped_text_get_descent(&self, _shaped: Rid) -> f64 {
        unreachable!()
    }

    fn shaped_text_get_width(&self, _shaped: Rid) -> f64 {
        unreachable!()
    }

    fn shaped_text_get_underline_position(&self, _shaped: Rid) -> f64 {
        unreachable!()
    }

    fn shaped_text_get_underline_thickness(&self, _shaped: Rid) -> f64 {
        unreachable!()
    }
}

#[itest]
fn native_structure_codegen() {
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
fn native_structure_out_parameter() {
    // Instantiate a TextServerExtension and then have Godot call a
    // function which uses an 'out' pointer parameter.
    let mut ext = TestTextServer::new_gd();
    let result = ext
        .clone()
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
        Some(Variant::from(Direction::AUTO))
    );
    assert_eq!(
        result.get("trailing_direction"),
        Some(Variant::from(Direction::LTR))
    );
}

#[itest]
fn native_structure_pointer_to_array_parameter() {
    // Instantiate a TextServerExtension.
    let ext = TestTextServer::new_gd();
    let result = ext
        .clone()
        .upcast::<TextServer>()
        .shaped_text_get_glyphs(Rid::new(100));

    // Check the result array.
    assert_eq!(result.len(), 2);
    assert_eq!(result.at(0).get("start"), Some(Variant::from(99)));
    assert_eq!(result.at(1).get("start"), Some(Variant::from(700)));
}

#[itest]
fn native_structure_clone() {
    // Instantiate CaretInfo directly.
    let caret1 = CaretInfo {
        leading_caret: Rect2::from_components(0.0, 0.0, 0.0, 0.0),
        trailing_caret: Rect2::from_components(1.0, 1.0, 1.0, 1.0),
        leading_direction: Direction::AUTO,
        trailing_direction: Direction::LTR,
    };

    // Clone a new CaretInfo.
    let caret2 = caret1.clone();

    // Test field-wise equality
    // between the original constructor arguments and the clone.
    assert_eq!(
        caret2.leading_caret,
        Rect2::from_components(0.0, 0.0, 0.0, 0.0)
    );
    assert_eq!(
        caret2.trailing_caret,
        Rect2::from_components(1.0, 1.0, 1.0, 1.0)
    );
    assert_eq!(caret2.leading_direction, Direction::AUTO);
    assert_eq!(caret2.trailing_direction, Direction::LTR);
}

#[itest]
fn native_structure_partialeq() {
    // Test basic equality between two identically-constructed
    // (but distinct) native structures.
    assert_eq!(sample_glyph(5), sample_glyph(5));
    assert_ne!(sample_glyph(1), sample_glyph(2));
}

#[itest]
fn native_structure_debug() {
    // Test debug output, both pretty-printed and not.
    let object_id = ObjectId { id: 256 };
    assert_eq!(
        format!("{:?}", object_id),
        String::from("ObjectId { id: 256 }")
    );
    assert_eq!(
        format!("{:#?}", object_id),
        String::from("ObjectId {\n    id: 256,\n}")
    );
}

#[itest]
fn native_structure_object_pointers() {
    let mut result = PhysicsServer2DExtensionShapeResult {
        rid: Rid::new(12),
        collider_id: ObjectId { id: 0 },
        raw_collider_ptr: std::ptr::null_mut(),
        shape: 0,
    };

    let retrieved = result.get_collider();
    assert_eq!(retrieved, None);

    let object = Node3D::new_alloc();
    result.set_collider(object.clone());
    assert_eq!(result.collider_id.id, object.instance_id().to_i64() as u64);

    let retrieved = result.get_collider();
    assert_eq!(retrieved, Some(object.clone().upcast()));

    object.free();
    assert_eq!(result.get_collider(), None);
}

#[itest(skip)] // Not yet implemented.
fn native_structure_refcounted_pointers() {
    let mut result = PhysicsServer2DExtensionShapeResult {
        rid: Rid::new(12),
        collider_id: ObjectId { id: 0 },
        raw_collider_ptr: std::ptr::null_mut(),
        shape: 0,
    };

    // RefCounted, increment ref-count.
    let object = RefCounted::new_gd();
    let id = object.instance_id();
    result.set_collider(object.clone());
    assert_eq!(result.collider_id.id, id.to_i64() as u64);

    drop(object); // Test if Godot keeps ref-count.

    let retrieved = result
        .get_collider()
        .expect("Ref-counted objects don't drop if ref-count is incremented");
    assert_eq!(retrieved.instance_id(), id);

    // Manually decrement refcount (method unexposed).
    //Gd::<RefCounted>::from_instance_id(id).call("unreference".into(), &[]);

    // RefCounted, do NOT increment ref-count.
    let object = RefCounted::new_gd();
    let id = object.instance_id();
    result.set_collider(object.clone());
    assert_eq!(result.collider_id.id, id.to_i64() as u64);

    drop(object); // Test if Godot keeps ref-count.

    let retrieved = result.get_collider();
    assert!(
        retrieved.is_none(),
        "Ref-counted objects drop if ref-count is not incremented"
    );
}
