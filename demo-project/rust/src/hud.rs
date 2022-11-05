use godot::engine::{Button, CanvasLayer, Label, Timer};
use godot::prelude::*;

#[derive(GodotClass)]
#[godot(base=CanvasLayer)]
pub struct Hud {
    base: Base<CanvasLayer>,
}

#[godot_api]
impl Hud {
    #[godot]
    pub fn show_message(&self, text: String) {
        let message_label = self.base.get_node_as::<Label>("message_label");
        message_label.set_text(text);
        message_label.show();

        let timer = self.base.get_node_as::<Timer>("message_timer");
        timer.start(0.0);
    }

    pub fn show_game_over(&self) {
        self.show_message("Game Over".into());

        let message_label = self.base.get_node_as::<Label>("message_label");
        message_label.set_text("Dodge the\nCreeps!");
        message_label.show();

        let button = self.base.get_node_as::<Button>("start_button");
        button.show();
    }

    #[godot]
    pub fn update_score(&self, score: i64) {
        let label = self.base.get_node_as::<Label>("score_label");
        label.set_text(score.to_string());
    }

    #[godot]
    fn on_start_button_pressed(&self) {
        let button = self.base.get_node_as::<Button>("start_button");
        button.hide();
        self.base.emit_signal("start_game", &[]);
    }

    #[godot]
    fn on_message_timer_timeout(&self) {
        let message_label = self.base.get_node_as::<Label>("message_label");
        message_label.hide()
    }
}

#[godot_api]
impl GodotExt for Hud {
    // TODO use signal once available
    // fn register_class(_builder: &mut ClassBuilder<Self>) {
    //     builder.signal("start_game").done();
    // }

    fn init(base: Base<Self::Base>) -> Self {
        Self { base }
    }
}
