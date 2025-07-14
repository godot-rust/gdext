/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::{vdict, vslice, Color, Dictionary, GString, Variant, VariantType};
use godot::classes::{INode, IRefCounted, Node, Object, RefCounted, Resource, Texture};
use godot::global::{PropertyHint, PropertyUsageFlags};
use godot::meta::{GodotConvert, PropertyHintInfo, ToGodot};
use godot::obj::{Base, EngineBitfield, EngineEnum, Gd, NewAlloc, NewGd, OnEditor};
use godot::register::property::{Export, Var};
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
    texture_val: OnEditor<Gd<Texture>>,

    #[var(get = get_texture_val, set = set_texture_val, hint = RESOURCE_TYPE, hint_string = "Texture")]
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
            texture_val: OnEditor::default(),
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
    fn export_hint() -> PropertyHintInfo {
        PropertyHintInfo {
            hint: PropertyHint::ENUM,
            hint_string: GString::from("A,B,C"),
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
        vdict! {
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
            A => GString::from("A"),
            B => GString::from("B"),
            C => GString::from("C"),
        }
    }
}

// These should all compile, but we can't easily test that they look right at the moment.
#[derive(GodotClass)]
#[class(no_init)]
struct CheckAllExports {
    #[export]
    normal: GString,

    #[export_group(name = "test_group", prefix = "a_")]
    #[export]
    a_grouped: i64,

    #[export]
    ungrouped_field_after_a: i64,

    #[export_subgroup(name = "some group")]
    #[export]
    subgrouped: i64,

    #[export_subgroup(name = "")]
    #[export]
    ungrouped: i64,

    // `suffix = "px"` should be in the third slot to test that key-value pairs in that position no longer error.
    #[export(range = (0.0, 10.0, suffix = "px", or_greater, or_less, exp, degrees, hide_slider))]
    range_exported: f64,

    #[export(range = (0.0, 10.0, 0.2, or_greater, or_less, exp, radians_as_degrees, hide_slider))]
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

#[derive(GodotConvert, Var)]
#[godot(via = i64)]
pub enum Behavior {
    Peaceful,
    Defend,
    Aggressive = (3 + 4),
}

#[derive(GodotConvert, Var)]
#[godot(via = GString)]
pub enum StrBehavior {
    Peaceful,
    Defend,
    Aggressive = (3 + 4),
}

#[derive(GodotClass)]
#[class(no_init)]
pub struct DeriveProperty {
    #[var]
    pub my_enum: TestEnum,
}

#[itest]
fn derive_property() {
    let mut class = DeriveProperty {
        my_enum: TestEnum::B,
    };
    assert_eq!(class.get_my_enum(), TestEnum::B as i64);

    class.set_my_enum(TestEnum::C as i64);
    assert_eq!(class.my_enum, TestEnum::C);
}

// Regression test for https://github.com/godot-rust/gdext/issues/1009.
#[itest]
fn enum_var_hint() {
    let int_prop = <Behavior as Var>::var_hint();
    assert_eq!(int_prop.hint, PropertyHint::ENUM);
    assert_eq!(
        int_prop.hint_string,
        "Peaceful:0,Defend:1,Aggressive:7".into()
    );

    let str_prop = <StrBehavior as Var>::var_hint();
    assert_eq!(str_prop.hint, PropertyHint::ENUM);
    assert_eq!(str_prop.hint_string, "Peaceful,Defend,Aggressive".into());
}

#[derive(GodotClass)]
pub struct DeriveExport {
    #[export]
    pub my_enum: TestEnum,

    // Tests also qualified base path (type inference of Base<T> without #[hint]).
    pub base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for DeriveExport {
    fn init(base: Base<Self::Base>) -> Self {
        Self {
            my_enum: TestEnum::B,
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
        .find(|c| c.get_or_nil("name") == "my_enum".to_variant())
        .unwrap();
    // `class_name` should be empty for non-Object variants.
    check_property(&property, "class_name", "");
    check_property(&property, "type", VariantType::INT.ord());
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
    pub my_resource: Option<Gd<CustomResource>>,

    #[export]
    pub renamed_resource: Option<Gd<RenamedCustomResource>>,
}

#[itest]
fn export_resource() {
    let class = ExportResource::new_alloc();

    let property = class
        .get_property_list()
        .iter_shared()
        .find(|c| c.get_or_nil("name") == "my_resource".to_variant())
        .unwrap();
    check_property(&property, "class_name", "CustomResource");
    check_property(&property, "type", VariantType::OBJECT.ord());
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
        .find(|c| c.get_or_nil("name") == "renamed_resource".to_variant())
        .unwrap();
    check_property(&property, "class_name", "NewNameCustomResource");
    check_property(&property, "type", VariantType::OBJECT.ord());
    check_property(&property, "hint", PropertyHint::RESOURCE_TYPE.ord());
    check_property(&property, "hint_string", "NewNameCustomResource");
    check_property(&property, "usage", PropertyUsageFlags::DEFAULT.ord());

    class.free();
}

#[derive(GodotClass)]
#[class(init)]
struct ExportOverride {
    #[export_group(name = "some group")]
    #[export]
    first: i32,

    #[export_group(name = "")]
    #[export]
    broke_out_of_some_group: i32,

    #[export_subgroup(name = "some subgroup", prefix = "b_")]
    #[export]
    b_second: i32,

    // This is really a nonsensical set of values, but they're different from what `#[export]` here would generate.
    // So we should be able to ensure that we can override the values `#[export]` generates.
    #[export]
    #[var(
        hint = GLOBAL_FILE,
        hint_string = "SomethingRandom",
        usage_flags = [GROUP],
    )]
    resource: Option<Gd<Resource>>,

    #[export]
    last: i32,
}

#[itest]
fn override_export() {
    let class = ExportOverride::new_gd();

    let property = class
        .get_property_list()
        .iter_shared()
        .find(|c| c.get_or_nil("name") == "resource".to_variant())
        .unwrap();

    check_property(&property, "hint", PropertyHint::GLOBAL_FILE.ord());
    check_property(&property, "hint_string", "SomethingRandom");
    check_property(&property, "usage", PropertyUsageFlags::GROUP.ord());
}

fn check_property(property: &Dictionary, key: &str, expected: impl ToGodot) {
    assert_eq!(property.get_or_nil(key), expected.to_variant());
}

// Checks if properties of a given class are arranged in the same order as ones declared in the Rust struct.
// Guaranteed order is necessary to make groups and subgroups work properly.
#[itest]
fn guaranteed_ordering() {
    let expected_order = [
        // Note: Category, displayed at the very top of the inspector.
        ("ExportOverride", PropertyUsageFlags::CATEGORY),
        ("some group", PropertyUsageFlags::GROUP),
        (
            "first",
            PropertyUsageFlags::EDITOR | PropertyUsageFlags::STORAGE,
        ),
        // Breaks out of some group.
        ("", PropertyUsageFlags::GROUP),
        (
            "broke_out_of_some_group",
            PropertyUsageFlags::EDITOR | PropertyUsageFlags::STORAGE,
        ),
        ("some subgroup", PropertyUsageFlags::SUBGROUP),
        (
            "b_second",
            PropertyUsageFlags::EDITOR | PropertyUsageFlags::STORAGE,
        ),
        ("resource", PropertyUsageFlags::GROUP),
        (
            "last",
            PropertyUsageFlags::EDITOR | PropertyUsageFlags::STORAGE,
        ),
        // Inherited from RefCounted – category and script.
        ("RefCounted", PropertyUsageFlags::CATEGORY),
        (
            "script",
            PropertyUsageFlags::NEVER_DUPLICATE
                | PropertyUsageFlags::EDITOR
                | PropertyUsageFlags::STORAGE,
        ),
    ];

    let class = ExportOverride::new_gd();
    let property_list = class.get_property_list();
    assert_eq!(property_list.len(), expected_order.len());

    for (idx, property) in property_list.iter_shared().enumerate() {
        let (Some(name), Some(usage)) = (
            property
                .get("name")
                .as_ref()
                .map(<String as godot::prelude::FromGodot>::from_variant),
            property
                .get("usage")
                .as_ref()
                .map(<PropertyUsageFlags as godot::prelude::FromGodot>::from_variant),
        ) else {
            panic!("Property dict should contain Property name and its usage.");
        };
        assert_eq!(name, expected_order[idx].0);
        assert_eq!(usage, expected_order[idx].1);
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(GodotClass)]
#[class(base=Node, init)]
struct RenamedFunc {
    #[var(get = get_int_val, set = set_int_val)]
    int_val: i32,
}

#[godot_api]
impl RenamedFunc {
    #[func(rename=f1)]
    pub fn get_int_val(&self) -> i32 {
        self.int_val
    }

    #[func(rename=f2)]
    pub fn set_int_val(&mut self, val: i32) {
        self.int_val = val;
    }
}

#[itest]
fn test_var_with_renamed_funcs() {
    let mut obj = RenamedFunc::new_alloc();

    assert_eq!(obj.bind().int_val, 0);
    assert_eq!(obj.bind().get_int_val(), 0);
    assert_eq!(obj.call("f1", &[]).to::<i32>(), 0);
    assert_eq!(obj.get("int_val").to::<i32>(), 0);

    obj.bind_mut().int_val = 42;

    assert_eq!(obj.bind().int_val, 42);
    assert_eq!(obj.bind().get_int_val(), 42);
    assert_eq!(obj.call("f1", &[]).to::<i32>(), 42);
    assert_eq!(obj.get("int_val").to::<i32>(), 42);

    obj.call("f2", vslice![84]);

    assert_eq!(obj.bind().int_val, 84);
    assert_eq!(obj.bind().get_int_val(), 84);
    assert_eq!(obj.call("f1", &[]).to::<i32>(), 84);
    assert_eq!(obj.get("int_val").to::<i32>(), 84);

    obj.set("int_val", &128.to_variant());

    assert_eq!(obj.bind().int_val, 128);
    assert_eq!(obj.bind().get_int_val(), 128);
    assert_eq!(obj.call("f1", &[]).to::<i32>(), 128);
    assert_eq!(obj.get("int_val").to::<i32>(), 128);

    obj.free();
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(GodotClass)]
#[class(init, base=Node)]
struct Duplicator {
    // #[export] would also make tests pass, but #[export(storage)] additionally hides the properties from the editor.
    // See https://docs.godotengine.org/en/stable/tutorials/scripting/gdscript/gdscript_exports.html#export-storage.
    #[export(storage)]
    int_export: i32,

    // Low-level #[var] should also work.
    #[var(usage_flags = [STORAGE])]
    int_var: i32,

    // Not copied because not marked as serialize ("storage").
    #[var]
    int_ignored: i32,

    #[export(storage)]
    optional_node: Option<Gd<Node>>,

    #[export(storage)]
    oneditor_node: OnEditor<Gd<Node>>,
}

#[itest]
fn test_duplicate_retains_properties() {
    let optional_node = Node::new_alloc();
    let oneditor_node = Node::new_alloc();

    // Set up original node.
    let mut original = Duplicator::new_alloc();
    {
        let mut original = original.bind_mut();
        original.int_export = 5;
        original.int_var = 7;
        original.int_ignored = 9; // Will not be copied.
        original.optional_node = Some(optional_node.clone());
        original.oneditor_node.init(oneditor_node.clone());
    }

    // Create duplicate and verify all properties are copied correctly.
    let duplicated: Gd<Duplicator> = original.duplicate().unwrap().cast();
    {
        let duplicated = duplicated.bind();
        assert_eq!(duplicated.int_export, 5);
        assert_eq!(duplicated.int_var, 7);
        assert_eq!(duplicated.int_ignored, 0); // Not copied.
        assert_eq!(duplicated.optional_node.as_ref().unwrap(), &optional_node);
        assert_eq!(&*duplicated.oneditor_node, &oneditor_node);
    }

    optional_node.free();
    oneditor_node.free();
    duplicated.free();
    original.free();
}
