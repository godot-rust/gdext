/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::{
    bind::property::ExportInfo,
    engine::{
        global::{PropertyHint, PropertyUsageFlags},
        Texture,
    },
    prelude::*,
    test::itest,
};

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
    string_val: GodotString,

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
    pub fn get_string_val(&self) -> GodotString {
        self.string_val.clone()
    }

    #[func]
    pub fn set_string_val(&mut self, val: GodotString) {
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
impl NodeVirtual for HasProperty {
    fn init(_base: Base<Node>) -> Self {
        HasProperty {
            int_val: 0,
            int_val_read: 2,
            int_val_write: 0,
            int_val_rw: 0,
            int_val_getter: 0,
            int_val_setter: 0,
            object_val: None,
            string_val: GodotString::new(),
            texture_val: Texture::new(),
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

impl Property for SomeCStyleEnum {
    type Intermediate = i64;

    fn get_property(&self) -> Self::Intermediate {
        (*self) as i64
    }

    fn set_property(&mut self, value: Self::Intermediate) {
        match value {
            0 => *self = Self::A,
            1 => *self = Self::B,
            2 => *self = Self::C,
            other => panic!("unexpected variant {other}"),
        }
    }
}

impl Export for SomeCStyleEnum {
    fn default_export_info() -> ExportInfo {
        ExportInfo {
            hint: PropertyHint::PROPERTY_HINT_ENUM,
            hint_string: "A,B,C".into(),
        }
    }
}

#[derive(Default)]
struct NotExportable {
    a: i64,
    b: i64,
}

impl Property for NotExportable {
    type Intermediate = Dictionary;

    fn get_property(&self) -> Self::Intermediate {
        dict! {
            "a": self.a,
            "b": self.b
        }
    }

    fn set_property(&mut self, value: Self::Intermediate) {
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
    fn enum_as_string(&self) -> GodotString {
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
struct CheckAllExports {
    #[export]
    normal: GodotString,

    #[export(range = (0.0, 10.0, or_greater, or_less, exp, radians, hide_slider))]
    range_exported: f64,

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
    file_no_filter: GodotString,

    #[export(file = "*.jpg")]
    file_filter: GodotString,

    #[export(global_file)]
    global_file_no_filter: GodotString,

    #[export(global_file = "*.txt")]
    global_file_filter: GodotString,

    #[export(dir)]
    dir: GodotString,

    #[export(global_dir)]
    global_dir: GodotString,

    #[export(multiline)]
    multiline: GodotString,

    #[export(placeholder = "placeholder")]
    placeholder: GodotString,

    #[export(color_no_alpha)]
    color_no_alpha: Color,
}

#[godot_api]
impl CheckAllExports {}

#[repr(i64)]
#[derive(Property, Debug, PartialEq, Eq, Export)]
pub enum TestEnum {
    A = 0,
    B = 1,
    C = 2,
}

#[derive(GodotClass)]
pub struct DeriveProperty {
    #[var]
    pub foo: TestEnum,
}

#[godot_api]
impl DeriveProperty {}

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

    #[base]
    pub base: Base<RefCounted>,
}

#[godot_api]
impl DeriveExport {}

#[godot_api]
impl RefCountedVirtual for DeriveExport {
    fn init(base: godot::obj::Base<Self::Base>) -> Self {
        Self {
            foo: TestEnum::B,
            base,
        }
    }
}

#[itest]
fn derive_export() {
    let class: Gd<DeriveExport> = Gd::new_default();

    let property = class
        .get_property_list()
        .iter_shared()
        .find(|c| c.get_or_nil("name") == "foo".to_variant())
        .unwrap();
    // `class_name` should be empty for non-Object variants.
    check_property(&property, "class_name", "");
    check_property(&property, "type", VariantType::Int as i32);
    check_property(&property, "hint", PropertyHint::PROPERTY_HINT_ENUM.ord());
    check_property(&property, "hint_string", "A:0,B:1,C:2");
    check_property(
        &property,
        "usage",
        PropertyUsageFlags::PROPERTY_USAGE_DEFAULT.ord(),
    );
}

#[derive(GodotClass)]
#[class(init, base=Resource)]
pub struct CustomResource {}

#[godot_api]
impl CustomResource {}

#[godot_api]
impl ResourceVirtual for CustomResource {}

#[derive(GodotClass)]
#[class(init, base=Resource, rename=NewNameCustomResource)]
pub struct RenamedCustomResource {}

#[godot_api]
impl RenamedCustomResource {}

#[godot_api]
impl ResourceVirtual for RenamedCustomResource {}

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct ExportResource {
    #[export]
    #[var(usage_flags=[PROPERTY_USAGE_DEFAULT, PROPERTY_USAGE_EDITOR_INSTANTIATE_OBJECT])]
    pub foo: Option<Gd<CustomResource>>,

    #[export]
    pub bar: Option<Gd<RenamedCustomResource>>,
}

#[godot_api]
impl ExportResource {}

#[godot_api]
impl NodeVirtual for ExportResource {}

#[itest]
fn export_resource() {
    let class: Gd<ExportResource> = Gd::new_default();

    let property = class
        .get_property_list()
        .iter_shared()
        .find(|c| c.get_or_nil("name") == "foo".to_variant())
        .unwrap();
    check_property(&property, "class_name", "CustomResource");
    check_property(&property, "type", VariantType::Object as i32);
    check_property(
        &property,
        "hint",
        PropertyHint::PROPERTY_HINT_RESOURCE_TYPE.ord(),
    );
    check_property(&property, "hint_string", "CustomResource");
    check_property(
        &property,
        "usage",
        PropertyUsageFlags::PROPERTY_USAGE_DEFAULT.ord()
            | PropertyUsageFlags::PROPERTY_USAGE_EDITOR_INSTANTIATE_OBJECT.ord(),
    );

    let property = class
        .get_property_list()
        .iter_shared()
        .find(|c| c.get_or_nil("name") == "bar".to_variant())
        .unwrap();
    check_property(&property, "class_name", "NewNameCustomResource");
    check_property(&property, "type", VariantType::Object as i32);
    check_property(
        &property,
        "hint",
        PropertyHint::PROPERTY_HINT_RESOURCE_TYPE.ord(),
    );
    check_property(&property, "hint_string", "NewNameCustomResource");
    check_property(
        &property,
        "usage",
        PropertyUsageFlags::PROPERTY_USAGE_DEFAULT.ord(),
    );

    class.free();
}

fn check_property(property: &Dictionary, key: &str, expected: impl ToVariant) {
    assert_eq!(property.get_or_nil(key), expected.to_variant());
}
