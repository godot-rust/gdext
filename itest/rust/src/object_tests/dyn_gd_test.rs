use crate::framework::itest;
use godot::prelude::*;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Test cases

#[itest]
fn dyn_gd_creation_bind() {
    // Tests whether type inference works even if object is never referenced.
    let _unused = dyn_gd!(Health; Gd::from_object(RefcHealth { hp: 1 }));

    let user_obj = RefcHealth { hp: 34 };
    let mut dyn_gd = dyn_gd!(Health; Gd::from_object(user_obj));

    {
        // Exclusive bind.
        let mut health = dyn_gd.dbind_mut();
        health.deal_damage(4);
    }
    {
        // Test multiple shared binds.
        let health_a = dyn_gd.dbind();
        let health_b = dyn_gd.dbind();

        assert_eq!(health_b.get_hitpoints(), 30);
        assert_eq!(health_a.get_hitpoints(), 30);
    }
    {
        let mut health = dyn_gd.dbind_mut();
        health.kill();

        assert_eq!(health.get_hitpoints(), 0);
    }
}

#[itest]
fn dyn_gd_creation_deref() {
    let node = foreign::NodeHealth::new_alloc();
    let original_id = node.instance_id();

    let mut node = dyn_gd!(Health; node);
    let dyn_id = node.instance_id();
    assert_eq!(dyn_id, original_id);

    deal_20_damage(&mut *node.dbind_mut());
    assert_eq!(node.dbind().get_hitpoints(), 80);

    node.free();
}

fn deal_20_damage(h: &mut dyn Health) {
    h.deal_damage(20);
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Example symbols

trait Health {
    fn get_hitpoints(&self) -> u8;

    fn deal_damage(&mut self, damage: u8);

    fn kill(&mut self) {
        self.deal_damage(self.get_hitpoints());
    }
}

#[derive(GodotClass)]
#[class(init)]
struct RefcHealth {
    hp: u8,
}

// Pretend NodeHealth is defined somewhere else, with a default constructor but
// no knowledge of health. We retrofit the property via Godot "meta" key-values.
mod foreign {
    use super::*;

    #[derive(GodotClass)]
    #[class(init, base=Node)]
    pub struct NodeHealth {
        base: Base<Node>,
    }
}

impl Health for RefcHealth {
    fn get_hitpoints(&self) -> u8 {
        self.hp
    }

    fn deal_damage(&mut self, damage: u8) {
        self.hp -= damage;
    }
}

impl Health for foreign::NodeHealth {
    fn get_hitpoints(&self) -> u8 {
        if self.base().has_meta("hp") {
            self.base().get_meta("hp").to::<u8>()
        } else {
            100 // initial value, if nothing set.
        }
    }

    fn deal_damage(&mut self, damage: u8) {
        let new_hp = self.get_hitpoints() - damage;
        self.base_mut().set_meta("hp", &new_hp.to_variant());
    }
}
