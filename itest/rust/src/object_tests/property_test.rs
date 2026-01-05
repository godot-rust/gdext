/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::{
    vdict, vslice, Color, GString, PackedInt32Array, VarDictionary, Variant, VariantType,
};
use godot::classes::{INode, IRefCounted, Node, Object, RefCounted, Resource};
use godot::global::{PropertyHint, PropertyUsageFlags};
use godot::meta::{GodotConvert, PropertyHintInfo, ToGodot};
use godot::obj::{Base, Gd, NewAlloc, NewGd, OnEditor};
use godot::register::property::{Export, Var};
use godot::register::{godot_api, Export, GodotClass, GodotConvert, Var};
use godot::test::itest;

#[derive(GodotClass)]
#[class(base=Node)]
struct HasProperty {
    #[var]
    string_val: GString,

    #[var(get = get_object_val, set = set_object_val)]
    object_val: Option<Gd<Object>>,

    #[var]
    resource_var: OnEditor<Gd<Resource>>,

    #[var(get = get_resource_rw, set = set_resource_rw, hint = RESOURCE_TYPE, hint_string = "Resource")]
    resource_rw: Option<Gd<Resource>>,

    #[var]
    packed_int_array: PackedInt32Array,

    #[var(pub, rename = renamed_variable)]
    unused_name: GString,
}

#[godot_api]
impl HasProperty {
    #[func]
    pub fn get_object_val(&self) -> Option<Gd<Object>> {
        self.object_val.clone()
    }

    #[func]
    pub fn set_object_val(&mut self, val: Option<Gd<Object>>) {
        self.object_val = val;
    }

    #[func]
    pub fn get_resource_rw(&self) -> Option<Gd<Resource>> {
        self.resource_rw.clone()
    }

    #[func]
    pub fn set_resource_rw(&mut self, val: Option<Gd<Resource>>) {
        self.resource_rw = val;
    }
}

#[godot_api]
impl INode for HasProperty {
    fn init(_base: Base<Node>) -> Self {
        HasProperty {
            string_val: GString::new(),
            object_val: None,
            resource_var: OnEditor::default(),
            resource_rw: None,
            packed_int_array: PackedInt32Array::new(),
            unused_name: GString::new(),
        }
    }
}

#[itest]
fn test_renamed_variable_reflection() {
    let mut obj = HasProperty::new_alloc();

    let prop_list = obj.get_property_list();
    assert!(prop_list
        .iter_shared()
        .any(|d| d.get("name") == Some("renamed_variable".to_variant())));
    assert!(!prop_list
        .iter_shared()
        .any(|d| d.get("name") == Some("unused_name".to_variant())));

    assert_eq!(obj.get("renamed_variable"), GString::new().to_variant());
    assert_eq!(obj.get("unused_name"), Variant::nil());

    let new_value = "variable changed".to_variant();
    obj.set("renamed_variable", &new_value);
    obj.set("unused_name", &"something different".to_variant());
    assert_eq!(obj.get("renamed_variable"), new_value);
    assert_eq!(obj.get("unused_name"), Variant::nil());

    obj.free();
}

#[itest]
fn test_renamed_variable_getter_setter() {
    let mut obj = HasProperty::new_alloc();
    obj.bind_mut()
        .set_renamed_variable(GString::from("changed"));

    assert!(obj.has_method("get_renamed_variable"));
    assert!(obj.has_method("set_renamed_variable"));
    assert!(!obj.has_method("get_unused_name"));
    assert!(!obj.has_method("get_unused_name"));
    assert_eq!(obj.bind().get_renamed_variable(), "changed");

    obj.free();
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
    type PubType = Self;

    fn var_get(field: &Self) -> Self::Via {
        (*field) as i64
    }

    fn var_set(field: &mut Self, value: Self::Via) {
        match value {
            0 => *field = Self::A,
            1 => *field = Self::B,
            2 => *field = Self::C,
            other => panic!("unexpected variant {other}"),
        }
    }

    fn var_pub_get(field: &Self) -> Self::PubType {
        *field
    }

    fn var_pub_set(field: &mut Self, value: Self::PubType) {
        *field = value;
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

#[derive(Default, Clone)]
struct NotExportable {
    a: i64,
    b: i64,
}

impl GodotConvert for NotExportable {
    type Via = VarDictionary;
}

impl Var for NotExportable {
    type PubType = Self;

    fn var_get(field: &Self) -> Self::Via {
        vdict! {
            "a": field.a,
            "b": field.b
        }
    }

    fn var_set(field: &mut Self, value: Self::Via) {
        let a = value.get("a").unwrap().to::<i64>();
        let b = value.get("b").unwrap().to::<i64>();

        field.a = a;
        field.b = b;
    }

    fn var_pub_get(field: &Self) -> Self::PubType {
        field.clone()
    }

    fn var_pub_set(field: &mut Self, value: Self::PubType) {
        *field = value;
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

// TODO(v0.5): consider if the below enums all need Clone -- they didn't in v0.4.
// Reason is that #[derive(Var)] implements Var::var_pub_get() in a way that requires cloning, effectively requiring Clone.
#[derive(GodotConvert, Var, Export, Clone, Eq, PartialEq, Debug)]
#[godot(via = i64)]
#[repr(i64)]
pub enum TestEnum {
    A = 0,
    B = 1,
    C = 2,
}

#[derive(Clone, GodotConvert, Var)]
#[godot(via = i64)]
pub enum Behavior {
    Peaceful,
    Defend,
    Aggressive = (3 + 4),
}

#[derive(Clone, GodotConvert, Var)]
#[godot(via = GString)]
pub enum StrBehavior {
    Peaceful,
    Defend,
    Aggressive = (3 + 4),
}

#[derive(GodotClass)]
#[class(no_init)]
pub struct EnumVars {
    #[var(pub)]
    pub my_enum: TestEnum,

    #[var]
    pub legacy_enum: TestEnum,
}

#[itest]
fn property_enum_var() {
    let mut obj = EnumVars {
        my_enum: TestEnum::B,
        legacy_enum: TestEnum::B,
    };

    // From v0.5 and #[var(pub)] getters/setters use the Rust type directly (not Via type).
    assert_eq!(obj.get_my_enum(), TestEnum::B);

    obj.set_my_enum(TestEnum::C);
    assert_eq!(obj.my_enum, TestEnum::C);
}

#[itest]
#[expect(deprecated)]
fn property_enum_var_legacy() {
    let mut obj = EnumVars {
        my_enum: TestEnum::B,
        legacy_enum: TestEnum::B,
    };

    assert_eq!(obj.get_legacy_enum(), TestEnum::B as i64);

    obj.set_legacy_enum(TestEnum::A as i64);
    assert_eq!(obj.legacy_enum, TestEnum::A);
}

// Regression test for https://github.com/godot-rust/gdext/issues/1009.
#[itest]
fn enum_var_hint() {
    let int_prop = <Behavior as Var>::var_hint();
    assert_eq!(int_prop.hint, PropertyHint::ENUM);
    assert_eq!(int_prop.hint_string, "Peaceful:0,Defend:1,Aggressive:7");

    let str_prop = <StrBehavior as Var>::var_hint();
    assert_eq!(str_prop.hint, PropertyHint::ENUM);
    assert_eq!(str_prop.hint_string, "Peaceful,Defend,Aggressive");
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
    check_property(&property, "type", VariantType::INT);
    check_property(&property, "hint", PropertyHint::ENUM);
    check_property(&property, "hint_string", "A:0,B:1,C:2");
    check_property(&property, "usage", PropertyUsageFlags::DEFAULT);
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
    check_property(&property, "type", VariantType::OBJECT);
    check_property(&property, "hint", PropertyHint::RESOURCE_TYPE);
    check_property(&property, "hint_string", "CustomResource");
    check_property(
        &property,
        "usage",
        PropertyUsageFlags::DEFAULT | PropertyUsageFlags::EDITOR_INSTANTIATE_OBJECT,
    );

    let property = class
        .get_property_list()
        .iter_shared()
        .find(|c| c.get_or_nil("name") == "renamed_resource".to_variant())
        .unwrap();
    check_property(&property, "class_name", "NewNameCustomResource");
    check_property(&property, "type", VariantType::OBJECT);
    check_property(&property, "hint", PropertyHint::RESOURCE_TYPE);
    check_property(&property, "hint_string", "NewNameCustomResource");
    check_property(&property, "usage", PropertyUsageFlags::DEFAULT);

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

    check_property(&property, "hint", PropertyHint::GLOBAL_FILE);
    check_property(&property, "hint_string", "SomethingRandom");
    check_property(&property, "usage", PropertyUsageFlags::GROUP);
}

fn check_property(property: &VarDictionary, key: &str, expected: impl ToGodot) {
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

// Tests that CoW packed-arrays' changes are reflected from Rust. See:
// * Rust (sync does work): https://github.com/godot-rust/gdext/pull/576
// * GDScript (not synced): https://github.com/godotengine/godot/issues/76150
#[itest]
fn test_copy_on_write_var() {
    let mut obj = HasProperty::new_alloc();

    // Mutate property via reflection -> verify change is reflected in Rust.
    obj.set(
        "packed_int_array",
        &PackedInt32Array::from([1, 2, 3]).to_variant(),
    );
    assert_eq!(
        obj.bind().packed_int_array,
        PackedInt32Array::from(&[1, 2, 3])
    );

    // Mutate property in Rust -> verify change is reflected in Godot.
    obj.bind_mut().packed_int_array.push(4);
    assert_eq!(
        obj.get("packed_int_array").to::<PackedInt32Array>(),
        PackedInt32Array::from(&[1, 2, 3, 4])
    );

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
