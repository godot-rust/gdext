use godot::prelude::*;

use super::multiple_impl_blocks_test::MultipleImplBlocks;

#[godot_api(secondary)]
impl MultipleImplBlocks {
    #[func]
    fn third(&self) -> String {
        "3rd result".to_string()
    }

    #[func]
    pub fn get_i32(&self) -> i32 {
        123
    }
}
