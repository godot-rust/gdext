use crate::framework::itest;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init)]
struct HasDefaultParameters {}

#[godot_api]
impl HasDefaultParameters {
    #[func]
    fn function_with_default_params(
        required: i32,
        #[opt(default = "test")] string: GString,
        #[opt(default = 123)] integer: i32,
        // #[opt(default=None)] object: Option<Gd<Node>>,
    ) -> VariantArray {
        varray![required, string, integer,]
    }
}

#[itest]
fn tests_default_parameters() {
    let mut obj = HasDefaultParameters::new_gd();
    let r = obj.call("function_with_default_params", &[0.to_variant()]);
    let r = r.to::<VariantArray>();
    assert_eq!(r, varray![0, "test", 123]);
}
