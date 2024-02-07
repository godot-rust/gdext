/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::meta::{GodotConvert, ToGodot};
use godot::builtin::{dict, Color, Dictionary, GString, Variant, VariantType};
use godot::engine::global::{PropertyHint, PropertyUsageFlags};
use godot::engine::{INode, IRefCounted, Node, Object, RefCounted, Texture};
use godot::obj::{Base, EngineBitfield, EngineEnum, Gd, NewAlloc, NewGd};
use godot::register::property::{Export, PropertyHintInfo, Var};
use godot::register::{godot_api, Export, GodotClass, GodotConvert, Var};
use godot::test::itest;

// No tests currently, tests using these classes are in Godot scripts.

#[derive(GodotClass)]
#[class(base=Node)]
struct HasProperty {
    #[var]
    int_val: i32,

    #[var(get = get_int_val_read)]
    int_val_read: i32,

    #[var(set = set_int_val_write)]
    int_val_write: i32,

    #[var(get = get_int_val_rw, set = set_int_val_rw)]
    int_val_rw: i32,

    #[var(get = get_int_val_getter, set)]
    int_val_getter: i32,

    #[var(get, set = set_int_val_setter)]
    int_val_setter: i32,

    #[var(get = get_string_val, set = set_string_val)]
    string_val: GString,

    #[var(get = get_object_val, set = set_object_val)]
    object_val: Option<Gd<Object>>,

    #[var]
    texture_val: Gd<Texture>,

    #[var(get = get_texture_val, set = set_texture_val, hint = PROPERTY_HINT_RESOURCE_TYPE, hint_string = "Texture")]
    texture_val_rw: Option<Gd<Texture>>,
}

#[godot_api]
impl HasProperty {
    #[func]
    pub fn get_int_val_read(&self) -> i32 {
        self.int_val_read
    }

    #[func]
    pub fn set_int_val_write(&mut self, val: i32) {
        self.int_val_write = val;
    }

    // Odd name to make sure it doesn't interfere with "get_*".
    #[func]
    pub fn retrieve_int_val_write(&mut self) -> i32 {
        self.int_val_write
    }

    #[func]
    pub fn get_int_val_rw(&self) -> i32 {
        self.int_val_rw
    }

    #[func]
    pub fn set_int_val_rw(&mut self, val: i32) {
        self.int_val_rw = val;
    }

    #[func]
    pub fn get_int_val_getter(&self) -> i32 {
        self.int_val_getter
    }

    #[func]
    pub fn set_int_val_setter(&mut self, val: i32) {
        self.int_val_setter = val;
    }

    #[func]
    pub fn get_string_val(&self) -> GString {
        self.string_val.clone()
    }

    #[func]
    pub fn set_string_val(&mut self, val: GString) {
        self.string_val = val;
    }

    #[func]
    pub fn get_object_val(&self) -> Variant {
        if let Some(object_val) = self.object_val.as_ref() {
            object_val.to_variant()
        } else {
            Variant::nil()
        }
    }

    #[func]
    pub fn set_object_val(&mut self, val: Gd<Object>) {
        self.object_val = Some(val);
    }

    #[func]
    pub fn get_texture_val_rw(&self) -> Variant {
        if let Some(texture_val) = self.texture_val_rw.as_ref() {
            texture_val.to_variant()
        } else {
            Variant::nil()
        }
    }

    #[func]
    pub fn set_texture_val_rw(&mut self, val: Gd<Texture>) {
        self.texture_val_rw = Some(val);
    }
}

#[godot_api]
impl INode for HasProperty {
    fn init(_base: Base<Node>) -> Self {
        HasProperty {
            int_val: 0,
            int_val_read: 2,
            int_val_write: 0,
            int_val_rw: 0,
            int_val_getter: 0,
            int_val_setter: 0,
            object_val: None,
            string_val: GString::new(),
            texture_val: Texture::new_gd(),
            texture_val_rw: None,
        }
    }
}

#[derive(Default, Copy, Clone)]
#[repr(i64)]
enum SomeCStyleEnum {
    #[default]
    A = 0,
    B = 1,
    C = 2,
}

impl GodotConvert for SomeCStyleEnum {
    type Via = i64;
}

impl Var for SomeCStyleEnum {
    fn get_property(&self) -> Self::Via {
        (*self) as i64
    }

    fn set_property(&mut self, value: Self::Via) {
        match value {
            0 => *self = Self::A,
            1 => *self = Self::B,
            2 => *self = Self::C,
            other => panic!("unexpected variant {other}"),
        }
    }
}

impl Export for SomeCStyleEnum {
    fn default_export_info() -> PropertyHintInfo {
        PropertyHintInfo {
            hint: PropertyHint::ENUM,
            hint_string: "A,B,C".into(),
        }
    }
}

#[derive(Default)]
struct NotExportable {
    a: i64,
    b: i64,
}

impl GodotConvert for NotExportable {
    type Via = Dictionary;
}

impl Var for NotExportable {
    fn get_property(&self) -> Self::Via {
        dict! {
            "a": self.a,
            "b": self.b
        }
    }

    fn set_property(&mut self, value: Self::Via) {
        let a = value.get("a").unwrap().to::<i64>();
        let b = value.get("b").unwrap().to::<i64>();

        self.a = a;
        self.b = b;
    }
}

#[derive(GodotClass)]
#[class(init)]
struct HasCustomProperty {
    #[export]
    some_c_style_enum: SomeCStyleEnum,
    #[var]
    not_exportable: NotExportable,
}

#[godot_api]
impl HasCustomProperty {
    #[func]
    fn enum_as_string(&self) -> GString {
        use SomeCStyleEnum::*;

        match self.some_c_style_enum {
            A => "A".into(),
            B => "B".into(),
            C => "C".into(),
        }
    }
}

// These should all compile, but we can't easily test that they look right at the moment.
#[derive(GodotClass)]
#[class(no_init)]
struct CheckAllExports {
    #[export]
    normal: GString,

    #[export(range = (0.0, 10.0, or_greater, or_less, exp, radians, hide_slider))]
    range_exported: f64,

    #[export(range = (0.0, 10.0, 0.2, or_greater, or_less, exp, radians, hide_slider))]
    range_exported_with_step: f64,

    #[export(enum = (A = 10, B, C, D = 20))]
    enum_exported: i64,

    #[export(exp_easing)]
    exp_easing_no_options: f64,

    #[export(exp_easing = (attenuation, positive_only))]
    exp_easing_with_options: f64,

    #[export(flags = (A = 1, B = 2, C = 4, D = 8, CD = 12, BC = 6))]
    flags: u32,

    #[export(flags_2d_physics)]
    flags_2d_physics: u32,

    #[export(flags_2d_render)]
    flags_2d_render: u32,

    #[export(flags_2d_navigation)]
    flags_2d_navigation: u32,

    #[export(flags_3d_physics)]
    flags_3d_physics: u32,

    #[export(flags_3d_render)]
    flags_3d_render: u32,

    #[export(flags_3d_navigation)]
    flags_3d_navigation: u32,

    #[export(file)]
    file_no_filter: GString,

    #[export(file = "*.jpg")]
    file_filter: GString,

    #[export(global_file)]
    global_file_no_filter: GString,

    #[export(global_file = "*.txt")]
    global_file_filter: GString,

    #[export(dir)]
    dir: GString,

    #[export(global_dir)]
    global_dir: GString,

    #[export(multiline)]
    multiline: GString,

    #[export(placeholder = "placeholder")]
    placeholder: GString,

    #[export(color_no_alpha)]
    color_no_alpha: Color,
}

#[derive(GodotConvert, Var, Export, Eq, PartialEq, Debug)]
#[godot(via = i64)]
#[repr(i64)]
pub enum TestEnum {
    A = 0,
    B = 1,
    C = 2,
}

#[derive(GodotClass)]
#[class(no_init)]
pub struct DeriveProperty {
    #[var]
    pub foo: TestEnum,
}

#[itest]
fn derive_property() {
    let mut class = DeriveProperty { foo: TestEnum::B };
    assert_eq!(class.get_foo(), TestEnum::B as i64);
    class.set_foo(TestEnum::C as i64);
    assert_eq!(class.foo, TestEnum::C);
}

#[derive(GodotClass)]
pub struct DeriveExport {
    #[export]
    pub foo: TestEnum,

    // Tests also qualified base path (type inference of Base<T> without #[hint]).
    pub base: godot::obj::Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for DeriveExport {
    fn init(base: godot::obj::Base<Self::Base>) -> Self {
        Self {
            foo: TestEnum::B,
            base,
        }
    }
}

#[itest]
fn derive_export() {
    let class = DeriveExport::new_gd();

    let property = class
        .get_property_list()
        .iter_shared()
        .find(|c| c.get_or_nil("name") == "foo".to_variant())
        .unwrap();
    // `class_name` should be empty for non-Object variants.
    check_property(&property, "class_name", "");
    check_property(&property, "type", VariantType::Int as i32);
    check_property(&property, "hint", PropertyHint::ENUM.ord());
    check_property(&property, "hint_string", "A:0,B:1,C:2");
    check_property(&property, "usage", PropertyUsageFlags::DEFAULT.ord());
}

#[derive(GodotClass)]
#[class(init, base=Resource)]
pub struct CustomResource {}

#[derive(GodotClass)]
#[class(init, base=Resource, rename=NewNameCustomResource)]
pub struct RenamedCustomResource {}

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct ExportResource {
    #[export]
    #[var(usage_flags=[DEFAULT, EDITOR_INSTANTIATE_OBJECT])]
    pub foo: Option<Gd<CustomResource>>,

    #[export]
    pub bar: Option<Gd<RenamedCustomResource>>,
}

#[itest]
fn export_resource() {
    let class = ExportResource::new_alloc();

    let property = class
        .get_property_list()
        .iter_shared()
        .find(|c| c.get_or_nil("name") == "foo".to_variant())
        .unwrap();
    check_property(&property, "class_name", "CustomResource");
    check_property(&property, "type", VariantType::Object as i32);
    check_property(&property, "hint", PropertyHint::RESOURCE_TYPE.ord());
    check_property(&property, "hint_string", "CustomResource");
    check_property(
        &property,
        "usage",
        PropertyUsageFlags::DEFAULT.ord() | PropertyUsageFlags::EDITOR_INSTANTIATE_OBJECT.ord(),
    );

    let property = class
        .get_property_list()
        .iter_shared()
        .find(|c| c.get_or_nil("name") == "bar".to_variant())
        .unwrap();
    check_property(&property, "class_name", "NewNameCustomResource");
    check_property(&property, "type", VariantType::Object as i32);
    check_property(&property, "hint", PropertyHint::RESOURCE_TYPE.ord());
    check_property(&property, "hint_string", "NewNameCustomResource");
    check_property(&property, "usage", PropertyUsageFlags::DEFAULT.ord());

    class.free();
}

fn check_property(property: &Dictionary, key: &str, expected: impl ToGodot) {
    assert_eq!(property.get_or_nil(key), expected.to_variant());
}
